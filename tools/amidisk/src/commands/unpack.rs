//! `amidisk <image> unpack <host-dir>` — extract the whole volume to a
//! host directory tree.

use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Host directory to extract into; created if it does not exist.
    pub host_dir: PathBuf,
}

pub fn run(_image: &Path, _args: Args) -> Result<()> {
    anyhow::bail!("not yet implemented")
}
