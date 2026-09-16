//! `amidisk <image> relabel <new-name>` — rename the volume itself,
//! via [`Mutator::relabel`](amiga_ffs::Mutator::relabel), which owns the
//! validation (30-byte classic limit on every variant, no `:` or `/`),
//! the disk-altered stamp, and the checksum.

use std::path::Path;
use std::time::SystemTime;

use amiga_ffs::populate::datestamp_from_system_time;
use anyhow::{Context, Result};
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// The volume's new name (1..=30 bytes, no `:` or `/`).
    pub new_name: String,
}

pub fn run(image: &Path, args: Args) -> Result<()> {
    let now = datestamp_from_system_time(SystemTime::now());
    let mut mutator = super::open_mutator(image)?.clock(now);

    mutator
        .relabel(args.new_name.as_bytes())
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("renaming {}", image.display()))?;

    super::save_back(mutator, image)?;
    println!("{}: renamed to {:?}", image.display(), args.new_name);
    Ok(())
}
