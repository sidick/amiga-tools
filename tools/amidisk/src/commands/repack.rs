//! `amidisk <image> repack <other-image>` — copy every entry from
//! another image into this one.

use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// The other disk image to copy entries from.
    pub other_image: PathBuf,
}

pub fn run(_image: &Path, _args: Args) -> Result<()> {
    anyhow::bail!("not yet implemented")
}
