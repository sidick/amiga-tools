//! `amirom list` — stub. `romtool list` enumerates the built-in
//! Remus/Romsplit catalog of known-splittable ROMs; `amiga-rom` ships no
//! such catalog (deliberately, per its README/PLAN.md — milestone 6 is
//! "derive module-boundary data without the restricted catalog" and is
//! not done yet). Real implementation is blocked on that milestone, or
//! on this crate bringing its own catalog format.

use anyhow::{bail, Result};
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Only list catalog entries whose name matches this glob (e.g. `Kick*`).
    #[arg(long)]
    pub rom_filter: Option<String>,

    /// Also list each entry's module breakdown.
    #[arg(long)]
    pub modules: bool,
}

pub fn run(_args: Args) -> Result<()> {
    bail!("amirom list: needs a module-boundary catalog in amiga-rom (no public split data yet)")
}
