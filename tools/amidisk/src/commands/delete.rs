//! `amidisk <image> delete <ami-path> [--recursive]` — delete a file, or
//! (with `--recursive`) a directory tree.
//!
//! Without `--recursive`, deleting a non-empty directory is refused by
//! `Mutator::delete` itself (`MutateError::DirectoryNotEmpty`); with it,
//! every child is deleted first (depth-first, so a directory is only ever
//! handed to `delete` once it is empty), then the target itself.

use std::path::Path;
use std::time::SystemTime;

use amiga_ffs::populate::datestamp_from_system_time;
use amiga_ffs::{EntryKind, Mutator};
use anyhow::{Context, Result};
use clap::Args as ClapArgs;

use crate::disk::FileDisk;

#[derive(ClapArgs)]
pub struct Args {
    /// Path inside the volume to delete.
    pub ami_path: String,

    /// Required to delete a non-empty directory; recurses.
    #[arg(long)]
    pub recursive: bool,
}

pub fn run(image: &Path, args: Args) -> Result<()> {
    let now = datestamp_from_system_time(SystemTime::now());
    let mut mutator = super::open_mutator(image)?.clock(now);

    let root = mutator.volume().root_lba();
    let entry = mutator
        .volume()
        .lookup_path(root, args.ami_path.as_bytes())
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("looking up {}", args.ami_path))?
        .with_context(|| format!("{}: not found", args.ami_path))?;

    if args.recursive && entry.kind == EntryKind::Directory {
        empty_out(&mut mutator, entry.lba)?;
    }

    mutator
        .delete(entry.parent as u64, &entry.name)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("deleting {}", args.ami_path))?;

    super::save_back(mutator, image)?;
    println!("{}: deleted", args.ami_path);
    Ok(())
}

/// Delete every entry inside `dir_lba`, recursing into subdirectories
/// first so each is empty by the time `delete` sees it. Hard/soft links
/// are deleted as themselves, never followed — the same "named but not
/// resolved" treatment `list` and `read` give them.
fn empty_out(mutator: &mut Mutator<FileDisk>, dir_lba: u64) -> Result<()> {
    let entries = mutator
        .volume()
        .read_dir(dir_lba)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("reading directory at block {dir_lba}"))?;

    for entry in entries {
        if entry.kind == EntryKind::Directory {
            empty_out(mutator, entry.lba)?;
        }
        mutator
            .delete(dir_lba, &entry.name)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .with_context(|| format!("deleting {}", super::display_name(&entry.name)))?;
    }
    Ok(())
}
