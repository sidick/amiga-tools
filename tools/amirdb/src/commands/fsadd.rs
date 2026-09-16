//! `amirdb <image> fsadd <host-binary> --dos-type <spec> [--version X.Y]`
//! — add a loadable filesystem driver. Stub: argument surface only.

use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Host file holding the driver binary (hunk format), to embed.
    pub host_binary: PathBuf,

    /// `fhb_DosType` this driver serves — `ofs`/`ffs[+intl][+dircache]`,
    /// `DOS0..DOS7`, `PDS3`-style, or `0x...`.
    #[arg(long = "dos-type")]
    pub dos_type: String,

    /// `fhb_Version` as `major.minor`, e.g. `1.2`.
    #[arg(long)]
    pub version: Option<String>,
}

pub fn run(_image: &Path, _block_size: usize, _args: Args) -> Result<()> {
    bail!("`fsadd` is not yet implemented")
}
