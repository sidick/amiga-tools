//! `amirdb <image> validate [--verbose]` — walk the RDB and report
//! structural problems: `Rdb::validate` (the chains and extents held in
//! memory) plus `validate_seg_lists` (the LSEG chains, which need the
//! disk back — see how `info` already calls it), grouped by kind, with
//! a scriptable exit status.
//!
//! Unlike `info`, which reports issues as a footnote on a summary of
//! the whole RDB, this command's whole job is the verdict — mirroring
//! amidisk's `validate`: exit 0 clean, 1 issues found, 2 the image
//! could not even be parsed (the one case `Rdb::validate` cannot cover,
//! since it needs a parsed `Rdb` to start from). `main`'s own signature
//! is `Result<()>`, so the non-zero codes are raised directly via
//! `std::process::exit` rather than threaded back through it.

use std::path::Path;

use amiga_rdb::ValidationIssue;
use anyhow::Result;
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Print every finding, not just the issues found. With a clean
    /// image, also print the per-kind counts the walk checked, so
    /// "clean" reads as a checked fact rather than an absence of
    /// output.
    #[arg(long)]
    pub verbose: bool,
}

/// A short label for grouping and printing, since `ValidationIssue`
/// carries no discriminant name of its own.
fn kind(issue: &ValidationIssue) -> &'static str {
    match issue {
        ValidationIssue::RdbAreaInvalid { .. } => "rdb area invalid",
        ValidationIssue::BlockOutsideRdbArea { .. } => "block outside rdb area",
        ValidationIssue::PartitionOverlapsRdbArea { .. } => "partition overlaps rdb area",
        ValidationIssue::PartitionCylindersInverted { .. } => "partition cylinders inverted",
        ValidationIssue::PartitionsOverlap { .. } => "partitions overlap",
        ValidationIssue::EnvecTooShort { .. } => "envec too short",
        ValidationIssue::SharedLsegChain { .. } => "shared LSEG chain",
    }
}

pub fn run(image: &Path, block_size: usize, args: Args) -> Result<()> {
    let (rdb, mut disk) = match super::open_rdb(image, block_size) {
        Ok(pair) => pair,
        Err(e) => {
            eprintln!("{}: could not open: {e:#}", image.display());
            std::process::exit(2);
        }
    };

    let mut issues = rdb.validate();
    // Kept apart from `issues` rather than folded in: `ValidationIssue`
    // is a closed enum this crate does not own, so a walk failure (a
    // cycle, an out-of-range LBA, a bad checksum on an LSEG block) has
    // nowhere to go inside it. Both halves count toward the exit status
    // and the summary line all the same.
    let mut seg_errors: Vec<String> = Vec::new();
    match rdb.validate_seg_lists(&mut disk) {
        Ok(more) => issues.extend(more),
        Err(e) => seg_errors.push(format!("walking LSEG chains: {e}")),
    }

    let total = issues.len() + seg_errors.len();

    if args.verbose || total > 0 {
        for issue in &issues {
            println!("! [{}] {issue}", kind(issue));
        }
        for e in &seg_errors {
            println!("! [lseg chain walk] {e}");
        }
    }

    if args.verbose {
        println!(
            "checked: {} partition(s), {} filesystem(s), {} bad block entr{}, rdb area {}..={}",
            rdb.partitions.len(),
            rdb.filesystems.len(),
            rdb.bad_blocks.len(),
            if rdb.bad_blocks.len() == 1 { "y" } else { "ies" },
            rdb.rdb_blocks_lo,
            rdb.rdb_blocks_hi,
        );
    }

    if total == 0 {
        println!("{}: clean", image.display());
        Ok(())
    } else {
        println!(
            "{}: {total} issue{}",
            image.display(),
            if total == 1 { "" } else { "s" }
        );
        std::process::exit(1);
    }
}
