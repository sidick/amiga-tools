//! `amidisk <image> time <ami-path> <timestamp>` — change an entry's
//! modification timestamp.

use std::path::Path;

use anyhow::Result;
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Path inside the volume.
    pub ami_path: String,

    /// New timestamp, amitools-xdftool style: `YYYY-MM-DD HH:MM:SS`.
    pub timestamp: String,
}

pub fn run(_image: &Path, _args: Args) -> Result<()> {
    anyhow::bail!("not yet implemented")
}
