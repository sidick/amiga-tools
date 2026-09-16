//! `amirom combine` — stub, but the one non-catalog-blocked stub here:
//! `amiga_rom::combine` already does exactly this (concatenate two
//! validated 512 KiB images), including a documented fix for
//! `romtool combine`'s reversed argument order. Left as a stub only
//! because Phase 1's scope is CLI-surface completion plus `dump`/`diff`;
//! wiring this one up is a five-line follow-up for Phase 2.

use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// First input (contributes the first half of the output).
    pub first: PathBuf,
    /// Second input (contributes the second half of the output).
    pub second: PathBuf,

    /// Where to write the combined 1 MiB image.
    #[arg(short, long)]
    pub out: PathBuf,
}

pub fn run(_args: Args) -> Result<()> {
    bail!("amirom combine: not yet implemented (amiga_rom::combine is ready; this is just unwired CLI plumbing)")
}
