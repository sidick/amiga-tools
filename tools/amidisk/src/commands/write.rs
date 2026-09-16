//! `amidisk <image> write <host-path> [ami-path] [--recursive]` — write a
//! host file (or, with `--recursive`, a directory tree) into the volume.

use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// File or directory on the host to write in.
    pub host_path: PathBuf,

    /// Destination inside the volume; defaults to the host path's own
    /// last component at the volume root.
    pub ami_path: Option<String>,

    /// Required to write a directory; copies it in recursively.
    #[arg(long)]
    pub recursive: bool,
}

pub fn run(_image: &Path, _args: Args) -> Result<()> {
    anyhow::bail!("not yet implemented")
}
