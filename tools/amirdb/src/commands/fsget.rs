//! `amirdb <image> fsget <dostype-or-index> <out-file>` — extract a
//! loadable filesystem driver's binary to a host file. Stub: argument
//! surface only.

use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Filesystem index, or a dostype spec (see `add --dos-type`).
    pub selector: String,

    /// Host file to write the driver binary to.
    pub out_file: PathBuf,
}

pub fn run(_image: &Path, _block_size: usize, _args: Args) -> Result<()> {
    bail!("`fsget` is not yet implemented")
}
