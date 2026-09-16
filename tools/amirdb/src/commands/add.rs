//! `amirdb <image> add <name> (--size <bytes> | --cyl <lo>-<hi>) [...]`
//! — add a partition to an existing RDB. Stub: argument surface only.

use std::path::Path;

use anyhow::{bail, Result};
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// `pb_DriveName` for the new partition.
    pub name: String,

    /// Size, e.g. `10Mi`, `500M`. Mutually exclusive with `--cyl`.
    #[arg(long, conflicts_with = "cyl")]
    pub size: Option<String>,

    /// Exact cylinder range, `lo-hi` (inclusive), e.g. `2-100`.
    #[arg(long)]
    pub cyl: Option<String>,

    /// `ofs`/`ffs[+intl][+dircache]`, `DOS0..DOS7`, `PDS3`-style, or
    /// `0x...`.
    #[arg(long = "dos-type", default_value = "ffs")]
    pub dos_type: String,

    /// Mark bootable.
    #[arg(long)]
    pub bootable: bool,

    /// `de_BootPri`; implies `--bootable`.
    #[arg(long = "boot-pri")]
    pub boot_pri: Option<i32>,

    /// Mount, but not automatically at boot.
    #[arg(long = "no-automount")]
    pub no_automount: bool,

    /// `de_MaxTransfer`.
    #[arg(long = "max-transfer")]
    pub max_transfer: Option<u32>,

    /// `de_NumBuffers`.
    #[arg(long = "num-buffers")]
    pub num_buffers: Option<u32>,

    /// `de_Reserved` — boot blocks at the start of the partition.
    #[arg(long)]
    pub reserved: Option<u32>,
}

pub fn run(_image: &Path, _block_size: usize, _args: Args) -> Result<()> {
    bail!("`add` is not yet implemented")
}
