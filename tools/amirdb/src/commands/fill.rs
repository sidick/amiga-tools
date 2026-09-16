//! `amirdb <image> fill [--name <name>] [--dos-type <spec>]` — add a
//! partition spanning every cylinder not yet claimed. Stub: argument
//! surface only.

use std::path::Path;

use anyhow::{bail, Result};
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// `pb_DriveName`; defaults to the first free `DH`*n*.
    #[arg(long)]
    pub name: Option<String>,

    /// `ofs`/`ffs[+intl][+dircache]`, `DOS0..DOS7`, `PDS3`-style, or
    /// `0x...`.
    #[arg(long = "dos-type", default_value = "ffs")]
    pub dos_type: String,
}

pub fn run(_image: &Path, _block_size: usize, _args: Args) -> Result<()> {
    bail!("`fill` is not yet implemented")
}
