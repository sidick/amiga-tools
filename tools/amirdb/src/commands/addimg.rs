//! `amirdb <image> addimg <host-image> [placement/name flags]` — add a
//! partition sized and placed as given, then copy a host image file
//! into it verbatim. Stub: argument surface only.

use std::path::Path;
use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Host file to copy into the new partition.
    pub host_image: PathBuf,

    /// `pb_DriveName`; defaults to the first free `DH`*n*.
    #[arg(long)]
    pub name: Option<String>,

    /// Size, e.g. `10Mi`, `500M`. Mutually exclusive with `--cyl`;
    /// defaults to the host image's own size.
    #[arg(long, conflicts_with = "cyl")]
    pub size: Option<String>,

    /// Exact cylinder range, `lo-hi` (inclusive).
    #[arg(long)]
    pub cyl: Option<String>,

    /// `ofs`/`ffs[+intl][+dircache]`, `DOS0..DOS7`, `PDS3`-style, or
    /// `0x...`.
    #[arg(long = "dos-type")]
    pub dos_type: Option<String>,

    /// Mark bootable.
    #[arg(long)]
    pub bootable: bool,

    /// `de_BootPri`; implies `--bootable`.
    #[arg(long = "boot-pri")]
    pub boot_pri: Option<i32>,

    /// Mount, but not automatically at boot.
    #[arg(long = "no-automount")]
    pub no_automount: bool,
}

pub fn run(_image: &Path, _block_size: usize, _args: Args) -> Result<()> {
    bail!("`addimg` is not yet implemented")
}
