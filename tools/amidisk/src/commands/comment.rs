//! `amidisk <image> comment <ami-path> <comment>` — change an entry's
//! filenote/comment. An empty string clears it.

use amiga_ffs::layout::COMMENT_MAX;
use std::path::Path;

use amiga_ffs::MetaUpdate;
use anyhow::{bail, Context, Result};
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Path inside the volume.
    pub ami_path: String,

    /// New comment text (empty string clears it).
    pub comment: String,
}

pub fn run(image: &Path, args: Args) -> Result<()> {
    if args.comment.len() > COMMENT_MAX {
        bail!("comment of {} bytes exceeds the {COMMENT_MAX}-byte limit", args.comment.len());
    }

    let mut mutator = super::open_mutator(image)?;
    let mut vol = mutator.volume();
    let root = vol.root_lba();
    let entry = vol
        .lookup_path(root, args.ami_path.as_bytes())
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("looking up {}", args.ami_path))?
        .with_context(|| format!("{}: not found", args.ami_path))?;

    mutator
        .set_metadata(entry.lba, &MetaUpdate::new().comment(args.comment.as_bytes()))
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("setting comment on {}", args.ami_path))?;

    super::save_back(mutator, image)?;

    if args.comment.is_empty() {
        println!("{}: comment cleared", args.ami_path);
    } else {
        println!("{}: {:?}", args.ami_path, args.comment);
    }
    Ok(())
}
