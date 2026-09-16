//! `amirdb <image> free` — report the cylinder ranges not claimed by any
//! partition. Stub: argument surface only.

use std::path::Path;

use anyhow::{bail, Result};
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {}

pub fn run(_image: &Path, _block_size: usize, _args: Args) -> Result<()> {
    bail!("`free` is not yet implemented")
}
