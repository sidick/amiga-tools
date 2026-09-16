//! `amidisk <image> resize (<new-size> | --min)` — grow or shrink the
//! volume in place.
//!
//! [`crate::disk::FileDisk`] implements [`amiga_ffs::ResizableMedium`], so
//! [`Volume::resize_to`] owns the whole job — extending the medium
//! before a grow, truncating it after a shrink — and the saved image is
//! simply the disk the volume hands back. A shrink refused with
//! [`ResizeError::RootTargetOccupied`] is retried once through
//! [`Volume::resize_evacuating`], which relocates whatever sits on the
//! new root's target block first; that path does not truncate the
//! medium itself, so it is the one place `set_block_count` is called by
//! hand.

use std::path::Path;

use amiga_ffs::{ResizableMedium, ResizeError};
use anyhow::{bail, Context, Result};
use clap::Args as ClapArgs;

use crate::disk::DEFAULT_BLOCK_SIZE;

#[derive(ClapArgs)]
pub struct Args {
    /// New size, e.g. `4M`; ignored (and required to be absent) with
    /// `--min`.
    pub new_size: Option<String>,

    /// Shrink to the smallest size that still holds every used block.
    #[arg(long, conflicts_with = "new_size")]
    pub min: bool,
}

pub fn run(image: &Path, args: Args) -> Result<()> {
    let bs = DEFAULT_BLOCK_SIZE as u64;

    let mut vol =
        super::open_volume(image).with_context(|| format!("could not open {}", image.display()))?;
    let old_block_count = vol.block_count();

    let new_block_count = if args.min {
        vol.minimum_size()
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("computing the minimum size")?
    } else {
        let spec = args
            .new_size
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("give a new size, e.g. 4M, or pass --min"))?;
        let bytes = super::parse_size(spec)?;
        if bytes % bs != 0 {
            bail!("{bytes} bytes is not a whole number of {bs}-byte blocks");
        }
        bytes / bs
    };

    if new_block_count == old_block_count {
        println!("{}: already {} bytes", image.display(), old_block_count * bs);
        return Ok(());
    }

    if new_block_count < old_block_count {
        let min = vol
            .minimum_size()
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("computing the minimum size")?;
        if new_block_count < min {
            bail!(
                "cannot shrink {} to {} bytes: the minimum is {} bytes ({} blocks)",
                image.display(),
                new_block_count * bs,
                min * bs,
                min
            );
        }
    }

    let vol = match vol.resize_to(new_block_count) {
        Ok(_) => vol,
        Err(ResizeError::RootTargetOccupied { .. }) => {
            let (mut vol, result) = vol.resize_evacuating(new_block_count);
            result
                .map_err(|e| anyhow::anyhow!("{e}"))
                .with_context(|| format!("resizing {} (evacuating)", image.display()))?;
            vol.source_mut()
                .set_block_count(new_block_count)
                .map_err(|e| anyhow::anyhow!("{e}"))
                .context("truncating the image")?;
            vol
        }
        Err(e) => {
            return Err(anyhow::anyhow!("{e}"))
                .with_context(|| format!("resizing {}", image.display()))
        }
    };

    debug_assert_eq!(vol.block_count(), new_block_count);
    let disk = vol.into_inner();
    disk.save(image).with_context(|| format!("writing {}", image.display()))?;

    println!(
        "{}: {} -> {} bytes ({} -> {} blocks)",
        image.display(),
        old_block_count * bs,
        new_block_count * bs,
        old_block_count,
        new_block_count
    );
    Ok(())
}
