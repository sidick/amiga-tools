use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args as ClapArgs;

use crate::commands::{load_rom, KeyArg};

#[derive(ClapArgs)]
pub struct Args {
    /// Raw ROM dump (any recognized byte order or Cloanto encoding).
    pub rom: PathBuf,
    /// Where to write the canonical (byte-order-normalized, decoded) image.
    pub out: PathBuf,

    #[command(flatten)]
    pub key: KeyArg,
}

pub fn run(args: Args) -> Result<()> {
    let key = args.key.load()?;
    let normalized = load_rom(&args.rom, key.as_deref())?;
    std::fs::write(&args.out, &normalized)
        .with_context(|| format!("writing {}", args.out.display()))?;
    println!("wrote {} bytes to {}", normalized.len(), args.out.display());
    Ok(())
}
