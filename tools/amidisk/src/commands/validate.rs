//! `amidisk <image> validate [--verbose]` — walk the volume and report
//! structural problems, without changing anything.
//!
//! Exit code doubles as the machine-readable answer: 0 clean, 1 findings,
//! 2 could not even open the volume (a bad checksum on the boot block, an
//! unreadable root, ...) — the one case `validate()` itself cannot walk
//! through, since it needs an opened `Volume` to start from. `main`'s own
//! signature is `Result<()>`, so the non-zero codes are raised directly
//! via `std::process::exit` rather than threaded back through it.

use std::path::Path;

use anyhow::Result;
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Print every finding, not just a summary. With a clean volume,
    /// also print the per-kind counts the walk collected.
    #[arg(long)]
    pub verbose: bool,
}

pub fn run(image: &Path, args: Args) -> Result<()> {
    let mut vol = match super::open_volume(image) {
        Ok(vol) => vol,
        Err(e) => {
            eprintln!("{}: could not open: {e:#}", image.display());
            std::process::exit(2);
        }
    };
    let report = vol.validate();

    if args.verbose || !report.is_clean() {
        for finding in &report.findings {
            println!("{finding}");
        }
        if report.truncated {
            println!(
                "... stopped after {} findings; there may be more",
                amiga_ffs::validate::MAX_FINDINGS
            );
        }
    }

    if args.verbose {
        let s = &report.summary;
        println!(
            "{} directories, {} files, {} hard links, {} soft links, {} data blocks, \
             {} extension blocks, {} dircache blocks, {} comment blocks, {} bitmap blocks",
            s.directories,
            s.files,
            s.hard_links,
            s.soft_links,
            s.data_blocks,
            s.extension_blocks,
            s.dircache_blocks,
            s.comment_blocks,
            s.bitmap_blocks
        );
        println!(
            "{} blocks reachable, {} allocated, {} free, {} orphaned, {} reachable-but-free",
            s.reachable, s.allocated, s.free, s.orphans, s.reachable_but_free
        );
    }

    if report.is_clean() {
        println!("{}: clean", image.display());
        Ok(())
    } else {
        println!(
            "{}: {} issue{}",
            image.display(),
            report.findings.len(),
            if report.findings.len() == 1 { "" } else { "s" }
        );
        std::process::exit(1);
    }
}
