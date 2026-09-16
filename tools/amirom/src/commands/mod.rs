//! One module per subcommand, mirroring `amidisk`/`amirdb`'s house
//! layout. This module holds what every command needs: loading a ROM
//! (raw read + `Loader::normalize`), the shared `--key` flag, and the
//! small formatting helpers several inspection commands share.

pub mod build;
pub mod combine;
pub mod copy;
pub mod diff;
pub mod dump;
pub mod eprom_merge;
pub mod eprom_split;
pub mod info;
pub mod list;
pub mod normalize;
pub mod patch;
pub mod patches;
pub mod query;
pub mod scan;
pub mod split;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Shared by every subcommand that accepts an optional Cloanto
/// `rom.key`. Kept here (not `main.rs`) so every command module can
/// `use crate::commands::KeyArg` without reaching back up to the crate
/// root.
#[derive(clap::Args)]
pub struct KeyArg {
    /// Cloanto/Amiga Forever `rom.key` file, required only if the input
    /// is Cloanto-encoded.
    #[arg(short, long)]
    pub key: Option<PathBuf>,
}

impl KeyArg {
    pub fn load(&self) -> Result<Option<Vec<u8>>> {
        parse_key_arg(self.key.as_deref())
    }
}

/// Reads a `rom.key` file if one was given. Split out from `KeyArg`
/// itself so a command that already has a bare `Option<&Path>` (e.g.
/// one composing two `KeyArg`s manually) can reuse it without a clap
/// struct in the way.
pub fn parse_key_arg(path: Option<&Path>) -> Result<Option<Vec<u8>>> {
    match path {
        Some(path) => {
            let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
            Ok(Some(bytes))
        }
        None => Ok(None),
    }
}

/// Reads a ROM file and normalizes it via `amiga_rom::Loader`, resolving
/// the optional Cloanto key first. Every subcommand that needs a
/// canonical image in memory goes through this one helper.
pub fn load_rom(path: &Path, key: Option<&[u8]>) -> Result<Vec<u8>> {
    let raw = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    amiga_rom::Loader::normalize(&raw, key).with_context(|| format!("normalizing {}", path.display()))
}

/// `0x1234ABCD` / `-` for header/footer fields that may be absent on a
/// too-short or invalid image. Shared by `info` and `dump`'s
/// annotations.
pub fn fmt_hex32(v: Option<u32>) -> String {
    match v {
        Some(v) => format!("0x{v:08X}"),
        None => "-".to_string(),
    }
}

/// `40.63` / `-` for a (major, minor) revision pair.
pub fn fmt_rev(v: Option<(u16, u16)>) -> String {
    match v {
        Some((maj, min)) => format!("{maj}.{min}"),
        None => "-".to_string(),
    }
}
