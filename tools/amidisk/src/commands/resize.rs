//! `amidisk <image> resize <new-size> [--min]` — grow or shrink the
//! volume in place (`Volume::resize`/`Mutator::resize_evacuating`).

use std::path::Path;

use anyhow::Result;
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// New size, e.g. `4M`; ignored (and required to be absent) with
    /// `--min`.
    pub new_size: Option<String>,

    /// Shrink to the smallest size that still holds every used block.
    #[arg(long, conflicts_with = "new_size")]
    pub min: bool,
}

pub fn run(_image: &Path, _args: Args) -> Result<()> {
    anyhow::bail!("not yet implemented")
}
