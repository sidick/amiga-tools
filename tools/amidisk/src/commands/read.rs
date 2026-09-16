//! `amidisk <image> read <amiga-path> [host-path]` — extract one file.

use std::path::{Path, PathBuf};

use amiga_ffs::{EntryKind, Volume};
use anyhow::{bail, Context, Result};

use crate::disk::FileDisk;

pub fn run(vol: &mut Volume<FileDisk>, amiga_path: &str, host_path: Option<PathBuf>) -> Result<()> {
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
    let dest = host_path.unwrap_or_else(|| {
        let base = amiga_path.rsplit('/').next().unwrap_or(amiga_path);
        PathBuf::from(base)
    });

    std::fs::write(&dest, &content)
        .with_context(|| format!("writing {}", Path::new(&dest).display()))?;
    println!("{amiga_path} -> {} ({} bytes)", dest.display(), content.len());
    Ok(())
}
