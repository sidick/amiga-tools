//! `amirom patches` — stub. `romtool patches` lists its built-in named
//! patches (e.g. `1mb_rom`). `amiga_rom::apply_patches` is a generic
//! find/verify/replace primitive with no bundled patch data (by design,
//! per its doc comment) — a named-patch table would live in this crate,
//! not upstream, once one exists.

use anyhow::{bail, Result};
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {}

pub fn run(_args: Args) -> Result<()> {
    bail!("amirom patches: not yet implemented (no named-patch table defined yet)")
}
