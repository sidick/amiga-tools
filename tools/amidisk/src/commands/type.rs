//! `amidisk <image> type <ami-path>` — print a file's contents to stdout.

use std::path::Path;

use anyhow::Result;
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Path inside the volume, e.g. `s/startup-sequence`.
    pub ami_path: String,
}

pub fn run(_image: &Path, _args: Args) -> Result<()> {
    anyhow::bail!("not yet implemented")
}
