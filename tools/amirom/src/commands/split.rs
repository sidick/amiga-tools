//! `amirom split` — stub. `romtool`'s module-splitting sense of "split"
//! (not to be confused with `eprom-split`, this crate's hi/lo EPROM
//! split): extracts a ROM's resident modules as LoadSeg()able binaries.
//! `amiga_rom::split` does the bounds-checked extraction, but needs
//! caller-supplied `ModuleSpec` ranges — this crate has no
//! module-boundary catalog to source them from yet (see `list.rs`).

use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::Args as ClapArgs;

use crate::commands::KeyArg;

#[derive(ClapArgs)]
pub struct Args {
    /// ROM image to split into modules.
    pub rom: PathBuf,

    #[command(flatten)]
    pub key: KeyArg,

    /// Directory to write extracted modules (and `index.txt`) into.
    #[arg(short, long)]
    pub out_dir: PathBuf,

    /// Only export modules whose name matches this glob.
    #[arg(long)]
    pub modules: Option<String>,

    /// Don't create a ROM-named subdirectory under `--out-dir`.
    #[arg(long)]
    pub no_version_dir: bool,

    /// Don't write an `index.txt` alongside the extracted modules.
    #[arg(long)]
    pub no_index: bool,
}

pub fn run(_args: Args) -> Result<()> {
    bail!("amirom split: not yet implemented (needs a module-boundary catalog; see amiga-rom's PLAN.md milestone 6)")
}
