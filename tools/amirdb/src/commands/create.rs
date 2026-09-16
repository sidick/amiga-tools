//! `amirdb <image> create (--size <bytes> | --chs <C,H,S>)` — write a
//! new, blank (unpartitioned, no RDB) image file. Refuses to overwrite
//! an existing one; `init` is the separate step that writes an RDB onto
//! the result.

use std::path::Path;

use anyhow::{bail, Context, Result};
use clap::Args as ClapArgs;

use crate::disk::FileDisk;

#[derive(ClapArgs)]
pub struct Args {
    /// Image size, e.g. `10Mi`, `500M`, `4G`. Mutually exclusive with
    /// `--chs`, whose geometry decides the size instead.
    #[arg(long, conflicts_with = "chs")]
    pub size: Option<String>,

    /// Exact geometry `cylinders,heads,sectors`; the image size becomes
    /// their product times the block size.
    #[arg(long)]
    pub chs: Option<String>,
}

/// Resolve `--size`/`--chs` to a byte count, at `block_size`. Shared with
/// `init --create`, which takes the same two flags.
pub fn resolve_bytes(args: &Args, block_size: usize) -> Result<u64> {
    match (&args.size, &args.chs) {
        (Some(s), None) => super::parse_size(s),
        (None, Some(chs)) => {
            let (c, h, s) = super::parse_geometry(chs)?;
            Ok(c as u64 * h as u64 * s as u64 * block_size as u64)
        }
        (None, None) => bail!("give --size or --chs"),
        (Some(_), Some(_)) => bail!("--size and --chs are mutually exclusive"),
    }
}

pub fn run(image: &Path, block_size: usize, args: Args) -> Result<()> {
    if image.exists() {
        bail!("{} already exists; refusing to overwrite it", image.display());
    }

    let bytes = resolve_bytes(&args, block_size)?;
    if block_size == 0 || bytes % block_size as u64 != 0 {
        bail!("{bytes} bytes is not a whole number of {block_size}-byte blocks");
    }
    let block_count = bytes / block_size as u64;

    let disk = FileDisk::new_zeroed(block_count, block_size);
    disk.save(image).with_context(|| format!("writing {}", image.display()))?;

    println!("{}: {bytes} bytes, {block_count} x {block_size}-byte blocks", image.display());
    Ok(())
}
