//! `amidisk <image> delete <ami-path> [--all]` — delete a file, or
//! (with `--all`) a directory tree.

use std::path::Path;

use anyhow::Result;
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Path inside the volume to delete.
    pub ami_path: String,

    /// Required to delete a non-empty directory; recurses.
    #[arg(long = "all")]
    pub all: bool,
}

pub fn run(_image: &Path, _args: Args) -> Result<()> {
    anyhow::bail!("not yet implemented")
}
