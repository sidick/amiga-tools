//! `amidisk <image> create (--size <bytes> | --adf | --hd) [--block-size <n>]`
//! — write a new, blank (unformatted) image file. Refuses to overwrite
//! an existing one; `format` is the separate step that makes it mountable.

use std::path::Path;

use anyhow::{bail, Context, Result};
use clap::Args as ClapArgs;

use crate::disk::{FileDisk, DEFAULT_BLOCK_SIZE};

/// A double-density floppy: 80 cylinders x 2 heads x 11 sectors x 512B.
pub const ADF_SIZE: u64 = 901_120;
/// A high-density floppy, as amitools-xdftool calls `--hd`: double the
/// sectors per track of `ADF_SIZE`.
pub const HD_SIZE: u64 = ADF_SIZE * 2;

#[derive(ClapArgs)]
pub struct Args {
    /// Image size, e.g. `880K`, `1760K`, `4M`. Mutually exclusive with
    /// `--adf`/`--hd`.
    #[arg(long, conflicts_with_all = ["adf", "hd"])]
    pub size: Option<String>,

    /// A standard 880K double-density floppy image.
    #[arg(long, conflicts_with = "hd")]
    pub adf: bool,

    /// A standard 1760K high-density floppy image.
    #[arg(long)]
    pub hd: bool,

    /// Block size in bytes (must be a power of two, 512..=32768).
    #[arg(long = "block-size", default_value_t = DEFAULT_BLOCK_SIZE)]
    pub block_size: usize,
}

/// Resolve `--size`/`--adf`/`--hd` (defaulting to `--adf`, amitools'
/// own default) to a byte count.
pub fn resolve_size(args: &Args) -> Result<u64> {
    match (&args.size, args.adf, args.hd) {
        (Some(s), false, false) => super::parse_size(s),
        (None, true, false) | (None, false, false) => Ok(ADF_SIZE),
        (None, false, true) => Ok(HD_SIZE),
        _ => bail!("--size, --adf and --hd are mutually exclusive"),
    }
}

pub fn run(image: &Path, args: Args) -> Result<()> {
    if image.exists() {
        bail!("{} already exists; refusing to overwrite it", image.display());
    }

    let bytes = resolve_size(&args)?;
    if args.block_size == 0 || bytes % args.block_size as u64 != 0 {
        bail!(
            "{bytes} bytes is not a whole number of {}-byte blocks",
            args.block_size
        );
    }
    let block_count = bytes / args.block_size as u64;

    let disk = FileDisk::new_zeroed(block_count, args.block_size);
    disk.save(image).with_context(|| format!("writing {}", image.display()))?;

    println!(
        "{}: {bytes} bytes, {block_count} x {}-byte blocks",
        image.display(),
        args.block_size
    );
    Ok(())
}
