//! `amidisk <image> read <amiga-path> [host-path]` — extract one file.
//!
//! Interim behaviour: files only. Recursing into a directory (per the
//! full spec) is left to a later worker.

use std::path::{Path, PathBuf};

use amiga_ffs::EntryKind;
use anyhow::{bail, Context, Result};
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Path inside the volume, e.g. `s/startup-sequence`.
    pub ami_path: String,

    /// Where to write it; defaults to the last path component in `.`.
    pub host_path: Option<PathBuf>,
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

    if entry.kind != EntryKind::File {
        bail!("{amiga_path}: not a plain file ({:?})", entry.kind);
    }

    let content = vol
        .read_file(entry.lba)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("reading {amiga_path}"))?;

    // Default destination: the Amiga path's own last component, taken
    // literally (case, accents and all) rather than re-derived from the
    // display string.
    let dest = args.host_path.unwrap_or_else(|| {
        let base = amiga_path.rsplit('/').next().unwrap_or(amiga_path);
        PathBuf::from(base)
    });

    std::fs::write(&dest, &content)
        .with_context(|| format!("writing {}", dest.display()))?;
    println!("{amiga_path} -> {} ({} bytes)", dest.display(), content.len());
    Ok(())
}
