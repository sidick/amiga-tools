//! `amidisk <image> resize (<new-size> | --min)` — grow or shrink the
//! volume in place (`Volume::resize`/`Volume::resize_evacuating`).
//!
//! `Volume::open` ties a volume's block count directly to its block
//! source's reported block count (`src.block_count()`), so growing needs
//! the underlying image bytes extended with zeros *before* the volume is
//! opened at its true (old) size via `Volume::open_with` -- otherwise
//! `resize`'s own writes near the new end have nowhere on the medium to
//! land. Shrinking needs no such preparation (the old buffer already has
//! room for everything up to the old, larger size), but does need the
//! saved image truncated to the new size afterwards, since `resize`
//! itself only ever writes blocks -- it never changes how many bytes the
//! backing `FileDisk` holds. Both paths end the same way: read back
//! exactly `new_block_count` blocks from the resized volume and hand
//! those bytes to a fresh `FileDisk` for `save`.

use std::path::Path;

use amiga_ffs::{BlockSource, ResizeError, Volume, DEFAULT_RESERVED};
use anyhow::{bail, Context, Result};
use clap::Args as ClapArgs;

use crate::disk::{FileDisk, DEFAULT_BLOCK_SIZE};

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

    // Opened read-only first, purely to learn the current size and (for
    // an explicit target) the minimum a shrink could reach -- neither
    // needs the write-side machinery below.
    let mut probe =
        super::open_volume(image).with_context(|| format!("could not open {}", image.display()))?;
    let old_block_count = probe.block_count();

    let new_block_count = if args.min {
        probe
            .minimum_size()
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
        let min = probe
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

    let vol = if new_block_count > old_block_count {
        // Grow: extend the raw bytes first, then open at the *old* block
        // count over the already-enlarged buffer -- see this module's
        // documentation for why `open_volume` alone cannot do this.
        drop(probe);
        let mut bytes = std::fs::read(image).with_context(|| format!("reading {}", image.display()))?;
        bytes.resize((new_block_count * bs) as usize, 0);
        let disk = FileDisk::new(bytes, DEFAULT_BLOCK_SIZE);
        Volume::open_with(disk, None, old_block_count, DEFAULT_RESERVED)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .with_context(|| format!("opening {} for growing", image.display()))?
    } else {
        probe
    };

    let (mut vol, result) = resize_or_evacuate(vol, new_block_count);
    result
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("resizing {}", image.display()))?;

    // `resize` only ever writes blocks; the backing buffer's own length
    // still reflects whichever size it was opened at. Read back exactly
    // the blocks the new volume claims and save those, so the image file
    // on disk matches `new_block_count` exactly, grown or shrunk.
    let mut block = vec![0u8; DEFAULT_BLOCK_SIZE];
    let mut bytes = Vec::with_capacity((new_block_count * bs) as usize);
    for lba in 0..new_block_count {
        vol.source_mut()
            .read_block(lba, &mut block)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .with_context(|| format!("reading back block {lba}"))?;
        bytes.extend_from_slice(&block);
    }
    let final_disk = FileDisk::new(bytes, DEFAULT_BLOCK_SIZE);
    final_disk.save(image).with_context(|| format!("writing {}", image.display()))?;

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

/// Try the plain resize; if it refuses only because the new root's target
/// block is occupied, retry once via `resize_evacuating`, which relocates
/// whatever is there first. Every other refusal (including
/// `EvacuationFailed`, `resize_evacuating`'s own name for the same
/// collision when it cannot be cleared) is returned as-is.
fn resize_or_evacuate(
    mut vol: Volume<FileDisk>,
    new_block_count: u64,
) -> (
    Volume<FileDisk>,
    Result<amiga_ffs::ResizeReport, ResizeError<<FileDisk as BlockSource>::Error>>,
) {
    match vol.resize(new_block_count) {
        Err(ResizeError::RootTargetOccupied { .. }) => vol.resize_evacuating(new_block_count),
        other => (vol, other),
    }
}
