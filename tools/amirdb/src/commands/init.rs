//! `amirdb <image> init [--create ...] [--rdsk-block <lba>] [--reserved <n>]`
//! — write a fresh, empty RDB (no partitions) onto an image, in place.
//!
//! `--create` is `create` and `init` fused into one call, the same way
//! amidisk's `format --create` fuses `create` and `format`: it takes
//! `create`'s own `--size`/`--chs` flags, refuses to overwrite an
//! existing image exactly as `create` does, then initializes the
//! result. Without `--create`, the image must already exist.

use std::path::Path;

use amiga_rdb::RdbBuilder;
use anyhow::{bail, Context, Result};
use clap::Args as ClapArgs;

use super::create;
use crate::disk::FileDisk;

#[derive(ClapArgs)]
pub struct Args {
    /// Create the image first, as `create` would, then initialize it.
    #[arg(long)]
    pub create: bool,

    /// Image size for `--create`, e.g. `10Mi`, `500M`, `4G`.
    #[arg(long, conflicts_with = "chs")]
    pub size: Option<String>,

    /// Geometry for `--create`, `cylinders,heads,sectors`.
    #[arg(long)]
    pub chs: Option<String>,

    /// Put the RDSK block somewhere other than block 0 (rare; matches
    /// `rdbtool`'s default of 0 otherwise).
    #[arg(long = "rdsk-block")]
    pub rdsk_block: Option<u32>,

    /// Reserve exactly this many blocks for the RDB area
    /// (`rdb_RDBBlocksLo..=rdb_RDBBlocksHi`), overriding the library's
    /// default sizing policy.
    #[arg(long)]
    pub reserved: Option<u32>,
}

pub fn run(image: &Path, block_size: usize, args: Args) -> Result<()> {
    let mut disk = if args.create {
        create::run(
            image,
            block_size,
            create::Args {
                size: args.size.clone(),
                chs: args.chs.clone(),
            },
        )?;
        FileDisk::load(image, block_size).with_context(|| format!("reading {}", image.display()))?
    } else {
        if args.size.is_some() || args.chs.is_some() {
            bail!("--size/--chs only apply together with --create");
        }
        if !image.exists() {
            bail!("{} does not exist; pass --create to make it first", image.display());
        }
        FileDisk::load(image, block_size).with_context(|| format!("reading {}", image.display()))?
    };

    let total_bytes = disk.len() as u64;
    let mut builder = RdbBuilder::for_size(total_bytes, block_size)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("choosing a geometry for {} bytes", total_bytes))?;
    if let Some(lba) = args.rdsk_block {
        builder = builder.rdsk_block(lba);
    }
    if let Some(blocks) = args.reserved {
        builder = builder.reserved_blocks(blocks);
    }

    let layout = builder
        .build(&mut disk)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("initializing an RDB on {}", image.display()))?;

    disk.save(image).with_context(|| format!("writing {}", image.display()))?;

    println!(
        "{}: RDB initialized, geometry {}/{}/{}, rdb blocks {}..={}, {} partitions",
        image.display(),
        layout.geometry.cylinders,
        layout.geometry.heads,
        layout.geometry.sectors,
        layout.rdb_blocks_lo,
        layout.rdb_blocks_hi,
        layout.partitions.len()
    );
    Ok(())
}
