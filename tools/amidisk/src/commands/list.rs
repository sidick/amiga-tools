//! `amidisk <image> list` — a recursive directory listing.

use amiga_ffs::{EntryKind, Volume};
use anyhow::{Context, Result};

use super::display_name;
use crate::disk::FileDisk;

pub fn run(vol: &mut Volume<FileDisk>) -> Result<()> {
    let name = display_name(&vol.root().name);
    println!("{name} [{:?}]", vol.variant());
    let root = vol.root_lba();
    walk(vol, root, "")
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
