//! `amirdb <image> change <name-or-index> [--set-... flags]` — change a
//! partition's mount parameters in place. One optional flag per settable
//! field; at least one must be given.

use std::path::Path;

use anyhow::{bail, Context, Result};
use clap::Args as ClapArgs;

use super::add::warn_if_not_clean;

/// `DosEnvec` longword indices for the three mount parameters
/// [`Partition`](amiga_rdb::Partition) does not surface as named fields
/// — `de_Reserved`/`de_PreAlloc`/`de_Interleave`, NDK `dos/filehandler.h`
/// order — read out of [`Partition::envec_raw`] for the before/after
/// summary.
const DE_RESERVED: usize = 6;
const DE_PRE_ALLOC: usize = 7;
const DE_INTERLEAVE: usize = 8;

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

    /// `pb_Flags` bit 0 — `true`/`false`.
    #[arg(long)]
    pub bootable: Option<bool>,

    /// `pb_Flags` bit 1, inverted — `true` mounts automatically at boot.
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

pub fn run(image: &Path, block_size: usize, args: Args) -> Result<()> {
    let anything = args.name.is_some()
        || args.dos_type.is_some()
        || args.boot_pri.is_some()
        || args.bootable.is_some()
        || args.automount.is_some()
        || args.reserved.is_some()
        || args.pre_alloc.is_some()
        || args.interleave.is_some()
        || args.num_buffers.is_some()
        || args.buf_mem_type.is_some()
        || args.max_transfer.is_some()
        || args.mask.is_some()
        || args.baud.is_some()
        || args.control.is_some()
        || args.boot_blocks.is_some();
    if !anything {
        bail!("`change` needs at least one --name/--dos-type/--boot-pri/... flag to change");
    }

    let (mut editor, disk) = super::open_editor(image, block_size)?;
    let index = super::find_partition(editor.rdb(), &args.selector)?;
    let before = editor.partitions()[index].clone();
    let mut changes: Vec<String> = Vec::new();

    if let Some(name) = &args.name {
        editor
            .set_name(index, name)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        changes.push(format!("name: {:?} -> {:?}", before.name, name));
    }
    if let Some(spec) = &args.dos_type {
        let v = super::parse_dostype(spec)?;
        editor.set_dos_type(index, v).map_err(|e| anyhow::anyhow!("{e}"))?;
        changes.push(format!("dos-type: {} -> {}", super::dostype_str(before.dos_type), super::dostype_str(v)));
    }
    if let Some(v) = args.boot_pri {
        editor.set_boot_priority(index, v).map_err(|e| anyhow::anyhow!("{e}"))?;
        changes.push(format!("boot-pri: {} -> {v}", before.boot_pri));
    }
    if let Some(v) = args.bootable {
        editor.set_bootable(index, v).map_err(|e| anyhow::anyhow!("{e}"))?;
        changes.push(format!("bootable: {} -> {v}", before.bootable));
    }
    if let Some(v) = args.automount {
        editor.set_automount(index, v).map_err(|e| anyhow::anyhow!("{e}"))?;
        changes.push(format!("automount: {} -> {v}", !before.no_automount));
    }
    if let Some(v) = args.reserved {
        editor.set_reserved(index, v).map_err(|e| anyhow::anyhow!("{e}"))?;
        changes.push(format!("reserved: {} -> {v}", before.envec_raw.get(DE_RESERVED).copied().unwrap_or(0)));
    }
    if let Some(v) = args.pre_alloc {
        editor.set_pre_alloc(index, v).map_err(|e| anyhow::anyhow!("{e}"))?;
        changes.push(format!("pre-alloc: {} -> {v}", before.envec_raw.get(DE_PRE_ALLOC).copied().unwrap_or(0)));
    }
    if let Some(v) = args.interleave {
        editor.set_interleave(index, v).map_err(|e| anyhow::anyhow!("{e}"))?;
        changes.push(format!("interleave: {} -> {v}", before.envec_raw.get(DE_INTERLEAVE).copied().unwrap_or(0)));
    }
    if let Some(v) = args.num_buffers {
        editor.set_num_buffers(index, v).map_err(|e| anyhow::anyhow!("{e}"))?;
        changes.push(format!("num-buffers: {} -> {v}", before.num_buffers));
    }
    if let Some(v) = args.buf_mem_type {
        editor.set_buf_mem_type(index, v).map_err(|e| anyhow::anyhow!("{e}"))?;
        changes.push(format!("buf-mem-type: 0x{:08x} -> 0x{v:08x}", before.buf_mem_type));
    }
    if let Some(v) = args.max_transfer {
        editor.set_max_transfer(index, v).map_err(|e| anyhow::anyhow!("{e}"))?;
        changes.push(format!("max-transfer: 0x{:08x} -> 0x{v:08x}", before.max_transfer));
    }
    if let Some(v) = args.mask {
        editor.set_mask(index, v).map_err(|e| anyhow::anyhow!("{e}"))?;
        changes.push(format!("mask: 0x{:08x} -> 0x{v:08x}", before.mask));
    }
    if let Some(v) = args.baud {
        editor.set_baud(index, v).map_err(|e| anyhow::anyhow!("{e}"))?;
        changes.push(format!("baud: {:?} -> {v}", before.baud));
    }
    if let Some(v) = args.control {
        editor.set_control(index, v).map_err(|e| anyhow::anyhow!("{e}"))?;
        changes.push(format!("control: {:?} -> {v}", before.control));
    }
    if let Some(v) = args.boot_blocks {
        editor.set_boot_blocks(index, v).map_err(|e| anyhow::anyhow!("{e}"))?;
        changes.push(format!("boot-blocks: {:?} -> {v}", before.boot_blocks));
    }

    super::commit_editor(&editor, disk, image)
        .with_context(|| format!("committing changes to {:?} on {}", args.selector, image.display()))?;

    println!("{}: changed {} ({} field(s)):", image.display(), before.name, changes.len());
    for change in &changes {
        println!("  {change}");
    }

    warn_if_not_clean(image, block_size)?;
    Ok(())
}
