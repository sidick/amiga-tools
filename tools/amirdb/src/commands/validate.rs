//! `amirdb <image> validate [--verbose]` — walk the RDB and report
//! structural problems. `info` already runs `Rdb::validate` /
//! `validate_seg_lists` and prints any issues found; this command is
//! for scripting (exit status, and eventually machine-readable output)
//! rather than a human skim. Stub: argument surface only.

use std::path::Path;

use anyhow::{bail, Result};
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Print every partition/filesystem/bad-block entry checked, not
    /// just the issues found.
    #[arg(long)]
    pub verbose: bool,
}

pub fn run(_image: &Path, _block_size: usize, _args: Args) -> Result<()> {
    bail!("`validate` is not yet implemented")
}
