//! `amidisk <image> makedir <ami-path>` — create a directory on the volume.

use std::path::Path;

use anyhow::Result;
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Path inside the volume to create, e.g. `s/new-dir`.
    pub ami_path: String,
}

pub fn run(_image: &Path, _args: Args) -> Result<()> {
    anyhow::bail!("not yet implemented")
}
