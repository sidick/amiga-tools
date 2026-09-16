//! `amirdb <image> fsdelete <selector>` — remove a loadable filesystem
//! driver, refusing (or, in phase 2, cascading) if a partition still
//! points at it. Stub: argument surface only.

use std::path::Path;

use anyhow::{bail, Result};
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Filesystem index, or a dostype spec (see `add --dos-type`).
    pub selector: String,
}

pub fn run(_image: &Path, _block_size: usize, _args: Args) -> Result<()> {
    bail!("`fsdelete` is not yet implemented")
}
