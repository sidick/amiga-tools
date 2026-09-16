//! `amidisk <image> repair [--sever] [--dry-run]` — rebuild the bitmap
//! and, with `--sever`, cut hash chains that walk into damage
//! (`amiga_ffs::repair::RepairOptions`).

use std::path::Path;

use amiga_ffs::RepairOptions;
use anyhow::{Context, Result};
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Cut out hash chains and comment pointers the reader has proved it
    /// cannot follow (a bad header block, a chain that loops). Off by
    /// default: severing throws away the only record of where the
    /// entries behind the cut were, and they stay allocated (leaked, not
    /// freed) either way. See `amiga_ffs::repair`'s own documentation for
    /// why this is the deliberately narrow, opt-in half of a repair.
    #[arg(long)]
    pub sever: bool,

    /// Validate and report what a repair would do, without writing
    /// anything back.
    #[arg(long = "dry-run")]
    pub dry_run: bool,
}

pub fn run(image: &Path, args: Args) -> Result<()> {
    let mut vol =
        super::open_volume(image).with_context(|| format!("could not open {}", image.display()))?;

    let before = vol.validate();
    println!(
        "before: {} issue{}",
        before.findings.len(),
        if before.findings.len() == 1 { "" } else { "s" }
    );
    if before.is_clean() {
        println!("{}: nothing to repair", image.display());
        return Ok(());
    }
    for finding in &before.findings {
        println!("  {finding}");
    }

    if args.dry_run {
        println!(
            "dry run: would rebuild the bitmap{}; nothing written",
            if args.sever { " and sever damaged chains" } else { "" }
        );
        return Ok(());
    }

    let opts = RepairOptions::new().sever(args.sever);
    let report = vol
        .repair(&opts)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("repairing {}", image.display()))?;
    println!(
        "repaired: {} block{} allocated, {} leak{} kept, {} bitmap page{} written, \
         {} block{} replaced, {} chain{}/pointer{} severed",
        report.allocated,
        if report.allocated == 1 { "" } else { "s" },
        report.leaked,
        if report.leaked == 1 { "" } else { "s" },
        report.pages_written,
        if report.pages_written == 1 { "" } else { "s" },
        report.blocks_replaced,
        if report.blocks_replaced == 1 { "" } else { "s" },
        report.severed,
        if report.severed == 1 { "" } else { "s" },
        if report.severed == 1 { "" } else { "s" },
    );

    let disk = vol.into_inner();
    disk.save(image).with_context(|| format!("writing {}", image.display()))?;

    // Re-validate the saved result, fresh from disk, so the before/after
    // counts describe what is actually on the medium now rather than
    // trusting the in-memory volume's own bookkeeping.
    let mut after_vol = super::open_volume(image)?;
    let after = after_vol.validate();
    println!(
        "after: {} issue{}",
        after.findings.len(),
        if after.findings.len() == 1 { "" } else { "s" }
    );
    Ok(())
}
