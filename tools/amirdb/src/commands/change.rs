//! `amirdb <image> change <name-or-index> [--set-... flags]` — change a
//! partition's mount parameters in place. Stub: argument surface only,
//! covering everything `RdbEditor` exposes a `set_*` for.

use std::path::Path;

use anyhow::{bail, Result};
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Partition index, or `pb_DriveName` (case-insensitive).
    pub selector: String,

    /// `pb_DriveName`.
    #[arg(long)]
    pub name: Option<String>,

    /// `de_DosType` — `ofs`/`ffs[+intl][+dircache]`, `DOS0..DOS7`,
    /// `PDS3`-style, or `0x...`.
    #[arg(long = "dos-type")]
    pub dos_type: Option<String>,

    /// `de_BootPri`.
    #[arg(long = "boot-pri")]
    pub boot_pri: Option<i32>,

    /// `pb_Flags` bit 0.
    #[arg(long)]
    pub bootable: Option<bool>,

    /// `pb_Flags` bit 1, inverted (`true` = automount).
    #[arg(long)]
    pub automount: Option<bool>,

    /// `de_Reserved`.
    #[arg(long)]
    pub reserved: Option<u32>,

    /// `de_PreAlloc`.
    #[arg(long = "pre-alloc")]
    pub pre_alloc: Option<u32>,

    /// `de_Interleave`.
    #[arg(long)]
    pub interleave: Option<u32>,

    /// `de_NumBuffers`.
    #[arg(long = "num-buffers")]
    pub num_buffers: Option<u32>,

    /// `de_BufMemType`.
    #[arg(long = "buf-mem-type")]
    pub buf_mem_type: Option<u32>,

    /// `de_MaxTransfer`.
    #[arg(long = "max-transfer")]
    pub max_transfer: Option<u32>,

    /// `de_Mask`.
    #[arg(long)]
    pub mask: Option<u32>,

    /// `de_BaudRate`.
    #[arg(long)]
    pub baud: Option<u32>,

    /// `de_Control`.
    #[arg(long)]
    pub control: Option<u32>,

    /// `de_BootBlocks`.
    #[arg(long = "boot-blocks")]
    pub boot_blocks: Option<u32>,
}

pub fn run(_image: &Path, _block_size: usize, _args: Args) -> Result<()> {
    bail!("`change` is not yet implemented")
}
