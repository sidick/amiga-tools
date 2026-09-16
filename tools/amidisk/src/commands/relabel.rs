//! `amidisk <image> relabel <new-name>` — rename the volume itself.

use std::path::Path;

use anyhow::Result;
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// The volume's new name.
    pub new_name: String,
}

pub fn run(_image: &Path, _args: Args) -> Result<()> {
    anyhow::bail!("not yet implemented")
}
