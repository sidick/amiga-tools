//! `amidisk <image> block <lba> [--hex]` — low-level single-block inspect.

use std::path::Path;

use anyhow::Result;
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Block number to dump.
    pub lba: u64,

    /// Hex-dump the block contents.
    #[arg(long)]
    pub hex: bool,
}

pub fn run(_image: &Path, _args: Args) -> Result<()> {
    anyhow::bail!("not yet implemented")
}
