//! `amidisk <image> format <name> [--dos-type <spec>] [--create ...]` —
//! write a fresh, empty filesystem onto an image, in place.
//!
//! `--create` is `create` and `format` fused into one call: it takes
//! the same size options as `create::Args` and refuses to overwrite an
//! existing image exactly as `create` does, then formats the result.
//! Without `--create`, the image must already exist; `--block-size`
//! still applies, since a raw (unformatted) image carries no block-size
//! metadata of its own for `FileDisk::load` to infer — it must be told,
//! and defaults to the 512 bytes every ADF and most HDF images use.

use std::path::Path;

use amiga_ffs::format::{self, FormatOptions};
use anyhow::{bail, Context, Result};
use clap::Args as ClapArgs;

use super::create;
use crate::disk::{FileDisk, DEFAULT_BLOCK_SIZE};

#[derive(ClapArgs)]
pub struct Args {
    /// The volume name to write into the root block (1..=30 bytes,
    /// no `:` or `/`).
    pub name: String,

    /// `ofs`, `ffs`, `ffs+intl`, `ffs+intl+dircache`, `DOS0`..`DOS7`,
    /// or a raw dostype like `0x444f5303`.
    #[arg(long = "dos-type", default_value = "ffs")]
    pub dos_type: String,

    /// Create the image first, as `create` would, then format it.
    #[arg(long)]
    pub create: bool,

    /// Image size for `--create`, e.g. `880K`, `1760K`, `4M`.
    #[arg(long, conflicts_with_all = ["adf", "hd"])]
    pub size: Option<String>,

    /// An 880K double-density floppy image, for `--create`.
    #[arg(long, conflicts_with = "hd")]
    pub adf: bool,

    /// A 1760K high-density floppy image, for `--create`.
    #[arg(long)]
    pub hd: bool,

    /// Block size in bytes, for `--create` or when reading an existing
    /// raw image that is not 512-byte-blocked.
    #[arg(long = "block-size", default_value_t = DEFAULT_BLOCK_SIZE)]
    pub block_size: usize,
}

pub fn run(image: &Path, args: Args) -> Result<()> {
    if args.name.is_empty() || args.name.len() > 30 || args.name.contains([':', '/']) {
        bail!("{:?} is not a valid volume name (1..=30 bytes, no ':' or '/')", args.name);
    }
    let variant = super::parse_variant(&args.dos_type)?;

    let mut disk = if args.create {
        let create_args = create::Args {
            size: args.size,
            adf: args.adf,
            hd: args.hd,
            block_size: args.block_size,
        };
        create::run(image, create_args)?;
        // Not `FileDisk::load`: that assumes 512-byte blocks, and the
        // image was just created at `args.block_size`.
        let data = std::fs::read(image).with_context(|| format!("reading {}", image.display()))?;
        FileDisk::new(data, args.block_size)
    } else {
        if args.size.is_some() || args.adf || args.hd {
            bail!("--size/--adf/--hd only apply together with --create");
        }
        if !image.exists() {
            bail!("{} does not exist; pass --create to make it first", image.display());
        }
        let data = std::fs::read(image).with_context(|| format!("reading {}", image.display()))?;
        FileDisk::new(data, args.block_size)
    };

    let block_count = disk.len() as u64 / disk.block_size_raw() as u64;
    let opts = FormatOptions::new(variant, block_count, args.name.as_bytes());
    let layout = format::format(&mut disk, &opts)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("formatting {}", image.display()))?;

    disk.save(image).with_context(|| format!("writing {}", image.display()))?;

    println!(
        "{}: {:?}, {} blocks, root at {}, {} blocks used",
        image.display(),
        variant,
        block_count,
        layout.root_lba,
        layout.blocks_used()
    );
    Ok(())
}
