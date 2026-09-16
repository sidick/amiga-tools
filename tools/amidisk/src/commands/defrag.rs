//! `amidisk <image> defrag [--dry-run] [--relocate-headers]` — reorganise
//! file data (tier 1) and, optionally, directory/file headers (tier 2)
//! for locality, via `amiga_ffs::compact`.

use std::path::Path;

use amiga_ffs::{CompactEvent, CompactOptions};
use anyhow::{Context, Result};
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Compute and report what would move, without writing it back.
    #[arg(long = "dry-run")]
    pub dry_run: bool,

    /// Also cluster directory and file headers back toward the root
    /// (tier 2). Off by default: on OFS this rewrites every data block
    /// of every relocated file, not just its header — see
    /// `amiga_ffs::compact`'s own documentation for why tier 1 alone is
    /// most of the benefit for a fraction of the cost.
    #[arg(long = "relocate-headers")]
    pub relocate_headers: bool,
}

pub fn run(image: &Path, args: Args) -> Result<()> {
    let opts = CompactOptions::new().dry_run(args.dry_run).relocate_headers(args.relocate_headers);

    // `CompactOptions::dry_run` computes the report without writing
    // anything -- but a `Mutator` session is still needed to run it (the
    // dry-run branch inside `compact_with` still reads chains via it), so
    // both paths open one; only the non-dry-run path ever calls
    // `save_back`.
    let mut mutator = super::open_mutator(image)
        .with_context(|| format!("could not open {} for writing", image.display()))?;

    // `compact_with`'s dry-run branch only ever tallies counts -- it never
    // invokes the progress callback, so `print_event` fires exclusively on
    // the real pass below, streaming one line per relocation as it
    // happens rather than after the fact.
    let report = mutator
        .compact_with(&opts, print_event)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("{} {}", if args.dry_run { "planning defrag of" } else { "defragmenting" }, image.display()))?;

    if args.dry_run {
        print_report(image, &report, true);
        return Ok(());
    }

    super::save_back(mutator, image)?;
    print_report(image, &report, false);
    Ok(())
}

fn print_event(event: CompactEvent) {
    match event {
        CompactEvent::FileDefragged(r) => println!(
            "file at block {}: {} run{} -> {} run{}, {} block{} relocated",
            r.header_lba,
            r.runs_before,
            if r.runs_before == 1 { "" } else { "s" },
            r.runs_after,
            if r.runs_after == 1 { "" } else { "s" },
            r.blocks_relocated,
            if r.blocks_relocated == 1 { "" } else { "s" },
        ),
        CompactEvent::HeaderRelocated(r) => println!(
            "header block {} -> {}: {} dependent{} relocated",
            r.old_lba,
            r.new_lba,
            r.dependents_relocated,
            if r.dependents_relocated == 1 { "" } else { "s" },
        ),
    }
}

fn print_report(image: &Path, report: &amiga_ffs::CompactReport, dry_run: bool) {
    println!(
        "{}: {} file{} examined, {} relocated{}, {} header{} relocated, {} -> {} run{}",
        image.display(),
        report.files_examined,
        if report.files_examined == 1 { "" } else { "s" },
        report.files_relocated,
        if dry_run { " (planned)" } else { "" },
        report.headers_relocated,
        if report.headers_relocated == 1 { "" } else { "s" },
        report.runs_before_total,
        report.runs_after_total,
        if report.runs_after_total == 1 { "" } else { "s" },
    );
}
