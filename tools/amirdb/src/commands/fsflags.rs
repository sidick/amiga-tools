//! `amirdb <image> fsflags <selector> <flags>` — set a filesystem
//! driver's `fhb_Flags`. Stub: argument surface only.

use std::path::Path;

use anyhow::{bail, Result};
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Filesystem index, or a dostype spec (see `add --dos-type`).
    pub selector: String,

    /// New `fhb_Flags`, e.g. `0x0` or a raw decimal value.
    pub flags: String,
}

pub fn run(_image: &Path, _block_size: usize, _args: Args) -> Result<()> {
    bail!("`fsflags` is not yet implemented")
}
