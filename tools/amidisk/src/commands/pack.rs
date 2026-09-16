//! `amidisk <image> pack <host-dir>` — populate the volume from a host
//! directory tree (`amiga_ffs::populate::populate_from_tree`, once the
//! volume already exists).

use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Host directory tree to copy in.
    pub host_dir: PathBuf,
}

pub fn run(_image: &Path, _args: Args) -> Result<()> {
    anyhow::bail!("not yet implemented")
}
