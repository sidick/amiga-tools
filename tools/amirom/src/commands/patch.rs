//! `amirom patch` — stub. Applies named patches (see `patches.rs`) or
//! explicit `offset=hex-bytes` edits via `amiga_rom::apply_patches`. The
//! explicit `--set` path is a thin wrapper once wired up; the named-patch
//! path is blocked on the same missing table as `patches.rs`.

use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::Args as ClapArgs;

use crate::commands::KeyArg;

#[derive(ClapArgs)]
pub struct Args {
    /// ROM image to patch.
    pub rom: PathBuf,

    /// Where to write the patched ROM image.
    #[arg(short, long)]
    pub out: PathBuf,

    #[command(flatten)]
    pub key: KeyArg,

    /// Named built-in patch to apply. Repeatable; see `amirom patches`.
    #[arg(long = "patch")]
    pub patches: Vec<String>,

    /// Explicit edit as `offset=hex-bytes`, e.g. `0x100=4e714e71`.
    /// Repeatable; combines with `--patch`.
    #[arg(long)]
    pub set: Vec<String>,
}

pub fn run(_args: Args) -> Result<()> {
    bail!("amirom patch: not yet implemented (named patches need patches.rs's table; --set needs wiring to amiga_rom::apply_patches)")
}
