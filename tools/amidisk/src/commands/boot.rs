//! `amidisk <image> boot <show|install|clear>` — boot block inspection
//! and (de)installation.

use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::{Args as ClapArgs, Subcommand};

#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    pub action: Action,
}

#[derive(Subcommand)]
pub enum Action {
    /// Print the boot block's dostype, root pointer and checksum status.
    Show,
    /// Install boot code (and a valid checksum) from a host file.
    Install {
        /// Boot code to install, e.g. a bootblock image.
        file: PathBuf,
    },
    /// Zero the boot code and checksum, leaving the volume non-bootable.
    Clear,
}

pub fn run(_image: &Path, _args: Args) -> Result<()> {
    anyhow::bail!("not yet implemented")
}
