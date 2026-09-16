//! `amirdb <image> remap (--geometry <C,H,S> | --auto)` — rewrite the
//! disk's geometry, recomputing every partition's cylinder range to
//! keep the same byte extents. Stub: argument surface only.

use std::path::Path;

use anyhow::{bail, Result};
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Exact geometry `cylinders,heads,sectors`.
    #[arg(long, conflicts_with = "auto")]
    pub geometry: Option<String>,

    /// Re-synthesize the geometry from the disk's current size, as
    /// `init` would for a fresh image.
    #[arg(long)]
    pub auto: bool,
}

pub fn run(_image: &Path, _block_size: usize, _args: Args) -> Result<()> {
    bail!("`remap` is not yet implemented")
}
