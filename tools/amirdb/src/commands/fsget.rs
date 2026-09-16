//! `amirdb <image> fsget <dostype-or-index> <out-file>` — extract a
//! loadable filesystem driver's binary to a host file.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Filesystem index, or a dostype spec (see `add --dos-type`).
    pub selector: String,

    /// Host file to write the driver binary to.
    pub out_file: PathBuf,
}

pub fn run(image: &Path, block_size: usize, args: Args) -> Result<()> {
    let (rdb, mut disk) = super::open_rdb(image, block_size)?;
    let index = super::find_filesystem(&rdb, &args.selector)?;
    let f = &rdb.filesystems[index];

    let binary = rdb
        .load_filesystem(f, &mut disk)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .context("reading LSEG chain")?;

    fs::write(&args.out_file, &binary).with_context(|| format!("writing {}", args.out_file.display()))?;

    println!(
        "wrote {} bytes  {}  version {}.{}  to {}",
        binary.len(),
        super::dostype_str(f.dos_type),
        f.version_major(),
        f.version_minor(),
        args.out_file.display(),
    );

    Ok(())
}
