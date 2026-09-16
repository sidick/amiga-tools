//! `amirdb <image> export <name-or-index> <host-file>` — copy a
//! partition's raw blocks out to a host file. Stub: argument surface
//! only.

use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Partition index, or `pb_DriveName` (case-insensitive).
    pub selector: String,

    /// Host file to write.
    pub host_file: PathBuf,
}

pub fn run(_image: &Path, _block_size: usize, _args: Args) -> Result<()> {
    bail!("`export` is not yet implemented")
}
