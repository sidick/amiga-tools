use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Hi bytes of an EPROM dump pair.
    pub hi: PathBuf,
    /// Lo bytes of an EPROM dump pair.
    pub lo: PathBuf,
    /// Where to write the merged canonical image.
    pub out: PathBuf,
}

/// No `--key` here: hi/lo dumps are already split at the canonical
/// byte-word level (see `amiga_rom::merge_hi_lo`'s doc comment) — any
/// Cloanto decoding or burner byte-swap happens on the merged result via
/// `normalize`, not before merging.
pub fn run(args: Args) -> Result<()> {
    let hi = std::fs::read(&args.hi).with_context(|| format!("reading {}", args.hi.display()))?;
    let lo = std::fs::read(&args.lo).with_context(|| format!("reading {}", args.lo.display()))?;

    let merged = amiga_rom::merge_hi_lo(&hi, &lo).context("merging hi/lo EPROM images")?;

    std::fs::write(&args.out, &merged)
        .with_context(|| format!("writing {}", args.out.display()))?;
    println!("wrote {} bytes to {}", merged.len(), args.out.display());
    Ok(())
}
