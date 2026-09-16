//! `amirdb <image> delete <name-or-index>` — remove a partition. Stub:
//! argument surface only.

use std::path::Path;

use anyhow::{bail, Result};
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Partition index, or `pb_DriveName` (case-insensitive).
    pub selector: String,
}

pub fn run(_image: &Path, _block_size: usize, _args: Args) -> Result<()> {
    bail!("`delete` is not yet implemented")
}
