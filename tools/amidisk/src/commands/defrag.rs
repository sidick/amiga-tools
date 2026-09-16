//! `amidisk <image> defrag [--dry-run]` — reorganise file data (and,
//! per `amiga_ffs::compact::CompactOptions`, optionally directory
//! headers) for locality.

use std::path::Path;

use anyhow::Result;
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Compute and report what would move, without writing it back.
    #[arg(long = "dry-run")]
    pub dry_run: bool,
}

pub fn run(_image: &Path, _args: Args) -> Result<()> {
    anyhow::bail!("not yet implemented")
}
