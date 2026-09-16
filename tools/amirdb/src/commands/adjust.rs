//! `amirdb <image> adjust [--rdb-blocks <n>] [--lo-cyl <n>]` — grow the
//! reserved RDB area and/or move the first cylinder available to
//! partitions. Stub: argument surface only.

use std::path::Path;

use anyhow::{bail, Result};
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// New `rdb_RDBBlocksHi + 1` — the reserved area's block count from
    /// block 0. Only grows the area; shrinking is refused by the
    /// library if it would clip a live structure.
    #[arg(long = "rdb-blocks")]
    pub rdb_blocks: Option<u32>,

    /// New `rdb_LoCylinder`.
    #[arg(long = "lo-cyl")]
    pub lo_cyl: Option<u32>,
}

pub fn run(_image: &Path, _block_size: usize, _args: Args) -> Result<()> {
    bail!("`adjust` is not yet implemented")
}
