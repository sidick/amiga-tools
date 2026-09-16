//! `amirdb <image> fill [--name <name>] [--dos-type <spec>] [...]` — add
//! a partition covering the largest free cylinder range, with the same
//! mount-parameter flags `add` takes.

use std::path::Path;

use amiga_rdb::PartitionSpec;
use anyhow::{Context, Result};
use clap::Args as ClapArgs;

use super::add::{apply_common_flags, describe, warn_if_not_clean, CommonFlags};
use super::free::free_ranges;

#[derive(ClapArgs)]
pub struct Args {
    /// `pb_DriveName`; defaults to the first free `DH`*n*.
    #[arg(long)]
    pub name: Option<String>,

    /// `ofs`/`ffs[+intl][+dircache]`, `DOS0..DOS7`, `PDS3`-style, or
    /// `0x...`. Defaults to `DOS3` (`ffs+intl`).
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

    /// `de_MaxTransfer`.
    #[arg(long = "max-transfer")]
    pub max_transfer: Option<u32>,

    /// `de_NumBuffers`.
    #[arg(long = "num-buffers")]
    pub num_buffers: Option<u32>,

    /// `de_Reserved` — boot blocks at the start of the partition.
    #[arg(long)]
    pub reserved: Option<u32>,
}

pub fn run(image: &Path, block_size: usize, args: Args) -> Result<()> {
    let (mut editor, disk) = super::open_editor(image, block_size)?;

    let (low, high) = free_ranges(editor.rdb())
        .into_iter()
        .max_by_key(|(low, high)| *high as u64 - *low as u64)
        .context("no free cylinders to fill")?;

    let mut base = PartitionSpec::by_cylinders(low, high);
    if let Some(name) = &args.name {
        base = base.named(name);
    }
    let spec = apply_common_flags(
        base,
        CommonFlags {
            dos_type: args.dos_type.as_deref(),
            bootable: args.bootable,
            boot_pri: args.boot_pri,
            no_automount: args.no_automount,
            max_transfer: args.max_transfer,
            num_buffers: args.num_buffers,
            reserved: args.reserved,
        },
    )?;

    let index = editor
        .add_partition(spec)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("filling {}..={} on {}", low, high, image.display()))?;
    let added = editor.partitions()[index].clone();

    super::commit_editor(&editor, disk, image)?;

    println!("{}: filled largest free range with {}", image.display(), describe(editor.rdb(), &added));

    warn_if_not_clean(image, block_size)?;
    Ok(())
}
