//! `amirom query` — stub. `romtool query` checks whether a given ROM
//! matches an entry in the module-boundary catalog (by KickSum) and, if
//! so, lists its modules; see `list.rs` for why this is blocked.

use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::Args as ClapArgs;

use crate::commands::KeyArg;

#[derive(ClapArgs)]
pub struct Args {
    /// ROM image to look up in the module-boundary catalog.
    pub rom: PathBuf,

    #[command(flatten)]
    pub key: KeyArg,

    /// Only show modules whose name matches this glob.
    #[arg(long)]
    pub modules: Option<String>,
}

pub fn run(_args: Args) -> Result<()> {
    bail!("amirom query: not yet implemented (needs a module-boundary catalog; see amiga-rom's PLAN.md milestone 6)")
}
