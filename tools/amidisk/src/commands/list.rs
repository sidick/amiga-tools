//! `amidisk <image> list [ami-path] [--all] [--info] [--detail]` — a
//! directory listing.
//!
//! Interim behaviour: always recurses from the root (or `ami-path`, if
//! given), regardless of `--all`; `--info`/`--detail` are accepted but
//! not yet acted on. A later worker replaces this with amitools-xdftool
//! parity (non-recursive by default, `--all` recurses, `--info` adds a
//! stats footer, `--detail` adds storage detail per entry).

use std::path::Path;

use amiga_ffs::{EntryKind, Volume};
use anyhow::{Context, Result};
use clap::Args as ClapArgs;

use super::display_name;
use crate::disk::FileDisk;

#[derive(ClapArgs)]
pub struct Args {
    /// Directory to list; defaults to the volume root.
    pub ami_path: Option<String>,

    /// Recurse into subdirectories.
    #[arg(long)]
    pub all: bool,

    /// Print a used/free stats footer.
    #[arg(long)]
    pub info: bool,

    /// Print storage detail (blocks, fragmentation) per entry.
    #[arg(long)]
    pub detail: bool,
}

pub fn run(image: &Path, args: Args) -> Result<()> {
    let mut vol = super::open_volume(image)?;

    let name = display_name(&vol.root().name);
    println!("{name} [{:?}]", vol.variant());

    let start = match &args.ami_path {
        Some(p) => {
            let root = vol.root_lba();
            let entry = vol
                .lookup_path(root, p.as_bytes())
                .map_err(|e| anyhow::anyhow!("{e}"))
                .with_context(|| format!("looking up {p}"))?
                .with_context(|| format!("{p}: not found"))?;
            if entry.kind != EntryKind::Directory {
                anyhow::bail!("{p}: not a directory ({:?})", entry.kind);
            }
            entry.lba
        }
        None => vol.root_lba(),
    };

    walk(&mut vol, start, "")
}

fn walk(vol: &mut Volume<FileDisk>, dir_lba: u64, prefix: &str) -> Result<()> {
    let entries = vol
        .read_dir(dir_lba)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("reading directory at block {dir_lba}"))?;
    for entry in entries {
        let name = display_name(&entry.name);
        match entry.kind {
            EntryKind::Directory => {
                println!("{prefix}{name}/");
                walk(vol, entry.lba, &format!("{prefix}{name}/"))?;
            }
            EntryKind::File => println!("{prefix}{name}  {} bytes", entry.byte_size),
            // Links: named but not followed, same as amitools' xdftool.
            other => println!("{prefix}{name}  [{other:?}]"),
        }
    }
    Ok(())
}
