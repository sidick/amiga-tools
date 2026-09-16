//! `amidisk <image> repair [--sever] [--dry-run]` — rebuild the bitmap
//! and, with `--sever`, cut hash chains that walk into damage
//! (`amiga_ffs::repair::RepairOptions`).

use std::path::Path;

use anyhow::Result;
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Also sever hash chains the walk cannot follow (off by default,
    /// as in `RepairOptions`).
    #[arg(long)]
    pub sever: bool,

    /// Report what would change without writing anything back.
    #[arg(long = "dry-run")]
    pub dry_run: bool,
}

pub fn run(_image: &Path, _args: Args) -> Result<()> {
    anyhow::bail!("not yet implemented")
}
