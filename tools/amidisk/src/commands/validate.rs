//! `amidisk <image> validate [--verbose]` — walk the volume and report
//! structural problems, without changing anything.

use std::path::Path;

use anyhow::Result;
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Print every finding, not just a summary.
    #[arg(long)]
    pub verbose: bool,
}

pub fn run(_image: &Path, _args: Args) -> Result<()> {
    anyhow::bail!("not yet implemented")
}
