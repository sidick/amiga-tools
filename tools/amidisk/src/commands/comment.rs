//! `amidisk <image> comment <ami-path> <comment>` — change an entry's
//! filenote/comment.

use std::path::Path;

use anyhow::Result;
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Path inside the volume.
    pub ami_path: String,

    /// New comment text (empty string clears it).
    pub comment: String,
}

pub fn run(_image: &Path, _args: Args) -> Result<()> {
    anyhow::bail!("not yet implemented")
}
