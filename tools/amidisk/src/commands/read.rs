//! `amidisk <image> read <amiga-path> [host-path] [--recursive]` —
//! extract one file, or (when the entry is a directory, or `--recursive`
//! is given) a whole subtree, to the host filesystem.

use std::path::{Path, PathBuf};

use amiga_ffs::{Entry, EntryKind, Volume};
use anyhow::{bail, Context, Result};
use clap::Args as ClapArgs;

use super::display_name;
use crate::disk::FileDisk;

#[derive(ClapArgs)]
pub struct Args {
    /// Path inside the volume, e.g. `s/startup-sequence`.
    pub ami_path: String,

    /// Where to write it; defaults to the last path component in `.`.
    pub host_path: Option<PathBuf>,

    /// Extract a directory's whole subtree. Implied when `ami-path` is
    /// itself a directory.
    #[arg(long)]
    pub recursive: bool,
}

pub fn run(image: &Path, args: Args) -> Result<()> {
    let mut vol = super::open_volume(image)?;
    let amiga_path = &args.ami_path;

    let root = vol.root_lba();
    let entry = vol
        .lookup_path(root, amiga_path.as_bytes())
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("looking up {amiga_path}"))?
        .with_context(|| format!("{amiga_path}: not found"))?;

    // Default destination: the Amiga path's own last component, taken
    // literally (case, accents and all) rather than re-derived from the
    // display string.
    let dest = args.host_path.clone().unwrap_or_else(|| {
        let base = amiga_path.rsplit('/').next().unwrap_or(amiga_path);
        PathBuf::from(base)
    });

    match entry.kind {
        EntryKind::File => {
            read_file(&mut vol, &entry, &dest)?;
        }
        EntryKind::Directory => {
            if !args.recursive {
                bail!("{amiga_path}: is a directory (use --recursive)");
            }
            let mut count = 0usize;
            read_tree(&mut vol, entry.lba, &dest, &mut count)?;
            println!("{amiga_path} -> {} ({count} entries)", dest.display());
        }
        other => bail!("{amiga_path}: not a plain file or directory ({other:?})"),
    }

    Ok(())
}

/// Extract one file's contents to `dest`.
fn read_file(vol: &mut Volume<FileDisk>, entry: &Entry, dest: &Path) -> Result<()> {
    let content = vol
        .read_file(entry.lba)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("reading {}", display_name(&entry.name)))?;

    std::fs::write(dest, &content).with_context(|| format!("writing {}", dest.display()))?;
    println!("{} -> {} ({} bytes)", display_name(&entry.name), dest.display(), content.len());
    Ok(())
}

/// Extract a directory's whole subtree into `dest`, creating it (and
/// every subdirectory) as needed. Names cross over via [`display_name`],
/// the same byte-to-char mapping `list` uses; hard/soft links are named
/// but not followed, same as `list`.
fn read_tree(vol: &mut Volume<FileDisk>, dir_lba: u64, dest: &Path, count: &mut usize) -> Result<()> {
    std::fs::create_dir_all(dest).with_context(|| format!("creating {}", dest.display()))?;

    let entries = vol
        .read_dir(dir_lba)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("reading directory at block {dir_lba}"))?;

    for entry in entries {
        let child_dest = dest.join(display_name(&entry.name));
        match entry.kind {
            EntryKind::Directory => {
                read_tree(vol, entry.lba, &child_dest, count)?;
            }
            EntryKind::File => {
                read_file(vol, &entry, &child_dest)?;
                *count += 1;
            }
            // Links: named but not followed, same as `list`.
            other => println!("{}  [{other:?}] (skipped)", display_name(&entry.name)),
        }
    }
    *count += 1;
    Ok(())
}
