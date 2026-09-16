use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args as ClapArgs;

use crate::commands::{load_rom, KeyArg};

#[derive(ClapArgs)]
pub struct Args {
    /// ROM image to split into an EPROM hi/lo byte-dump pair.
    pub rom: PathBuf,
    /// Output path for the hi bytes.
    pub hi_out: PathBuf,
    /// Output path for the lo bytes.
    pub lo_out: PathBuf,

    #[command(flatten)]
    pub key: KeyArg,
}

pub fn run(args: Args) -> Result<()> {
    let key = args.key.load()?;
    let normalized = load_rom(&args.rom, key.as_deref())?;
    let (hi, lo) = amiga_rom::split_hi_lo(&normalized).context("splitting hi/lo EPROM images")?;

    std::fs::write(&args.hi_out, &hi)
        .with_context(|| format!("writing {}", args.hi_out.display()))?;
    std::fs::write(&args.lo_out, &lo)
        .with_context(|| format!("writing {}", args.lo_out.display()))?;

    println!(
        "wrote {} bytes to {} and {} bytes to {}",
        hi.len(),
        args.hi_out.display(),
        lo.len(),
        args.lo_out.display()
    );
    Ok(())
}
