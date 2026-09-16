//! One module per subcommand. Add a new file + a `mod` line here plus a
//! `Command` variant in `main.rs` to extend the CLI.

pub mod info;
pub mod merge;
pub mod normalize;
pub mod scan;
pub mod split;

use std::path::Path;

use anyhow::{Context, Result};

/// Reads a ROM file and normalizes it via `amiga_rom::Loader`, resolving
/// the optional Cloanto key first. Every subcommand that needs a
/// canonical image in memory goes through this one helper.
pub fn load_and_normalize(path: &Path, key: Option<&[u8]>) -> Result<Vec<u8>> {
    let raw = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    amiga_rom::Loader::normalize(&raw, key)
        .with_context(|| format!("normalizing {}", path.display()))
}
