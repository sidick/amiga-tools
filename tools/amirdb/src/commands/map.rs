//! `amirdb <image> map` — show the block-level layout of the RDB area
//! and every partition. Stub: argument surface only.

use std::path::Path;

use anyhow::{bail, Result};
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {}

pub fn run(_image: &Path, _block_size: usize, _args: Args) -> Result<()> {
    bail!("`map` is not yet implemented")
}
