//! `amirdb <image> import <name-or-index> <host-file>` — copy a host
//! file into a partition's raw blocks verbatim. Stub: argument surface
//! only.

use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Partition index, or `pb_DriveName` (case-insensitive).
    pub selector: String,

    /// Host file to read.
    pub host_file: PathBuf,
}

pub fn run(_image: &Path, _block_size: usize, _args: Args) -> Result<()> {
    bail!("`import` is not yet implemented")
}
