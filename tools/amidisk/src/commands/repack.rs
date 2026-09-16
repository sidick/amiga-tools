//! `amidisk <image> repack <src-image> [--size <spec>] [--force]` —
//! read `<src-image>` and rebuild `image` through a fresh
//! `amiga_ffs::Populator`, entirely in memory: same variant, volume
//! name, creation date and boot block, every entry's protection,
//! comment, date and owner carried across exactly — but written out
//! through the populator's own two-cursor allocation instead of
//! whatever layout accumulated on the source, so the result is
//! defragmented and compactly laid out.
//!
//! Entries are copied in the reverse of `read_dir`'s own order, per
//! directory — see `unpack`'s module documentation for why that is what
//! reproduces the source's exact hash-chain (and so `list`) order
//! rather than reversing it.
//!
//! Target size defaults to the source's own; `--size` may grow or
//! shrink it, and a shrink that no longer fits fails with the
//! `Populator`'s own refusal rather than a truncated image.
//!
//! Hard and soft links have no representation `amiga_ffs::Populator`
//! can create (its write surface is directories and files only), so an
//! image containing one is refused rather than silently dropped.

use std::path::{Path, PathBuf};

use amiga_ffs::{EntryKind, FormatOptions, Metadata, Populator, Volume};
use anyhow::{anyhow, bail, Context, Result};
use clap::Args as ClapArgs;

use super::display_name;
use super::unpack;
use crate::disk::{DiskError, FileDisk, DEFAULT_BLOCK_SIZE};

#[derive(ClapArgs)]
pub struct Args {
    /// The source disk image to rebuild from.
    pub src_image: PathBuf,

    /// Target size, e.g. `880K`, `1760K`, `4M`. Defaults to the
    /// source's own size.
    #[arg(long)]
    pub size: Option<String>,

    /// Overwrite `image` if it already exists.
    #[arg(long)]
    pub force: bool,
}

pub fn run(image: &Path, args: Args) -> Result<()> {
    if image.exists() && !args.force {
        bail!("{} already exists; pass --force to overwrite it", image.display());
    }

    let mut src = super::open_volume(&args.src_image)?;
    let variant = src.variant();
    let name = src.root().name.clone();
    let created = src.root().disk_made;
    let src_root = src.root_lba();

    // Captured before the tree walk, which needs `src` mutably too.
    let boot_area = unpack::read_boot_area(src.source_mut())?;
    let boot_meaningful = unpack::boot_is_meaningful(&boot_area);

    let target_bytes = match &args.size {
        Some(s) => super::parse_size(s)?,
        None => src.block_count() * DEFAULT_BLOCK_SIZE as u64,
    };
    if target_bytes % DEFAULT_BLOCK_SIZE as u64 != 0 {
        bail!("{target_bytes} bytes is not a whole number of {DEFAULT_BLOCK_SIZE}-byte blocks");
    }
    let block_count = target_bytes / DEFAULT_BLOCK_SIZE as u64;

    let disk = FileDisk::new_zeroed(block_count, DEFAULT_BLOCK_SIZE);
    let opts = FormatOptions::new(variant, block_count, &name).created(created);
    let mut pop = Populator::new(disk, &opts)
        .map_err(|e| anyhow!("{e}"))
        .with_context(|| format!("formatting a {target_bytes}-byte target"))?;
    let dst_root = pop.root_lba();

    copy_dir(&mut src, &mut pop, src_root, dst_root)
        .with_context(|| "the source's contents did not fit the target size".to_string())?;

    let mut disk = pop.finish().map_err(|e| anyhow!("{e}"))?;
    if boot_meaningful {
        unpack::write_bootblock(&mut disk, &boot_area, variant)?;
    }
    disk.save(image).with_context(|| format!("writing {}", image.display()))?;

    println!(
        "{} <- {} ({:?}, {target_bytes} bytes)",
        image.display(),
        args.src_image.display(),
        variant
    );
    Ok(())
}

/// Copy one directory's entries from `src` into `dst`, recursively, in
/// the reverse of `read_dir`'s own order.
fn copy_dir(src: &mut Volume<FileDisk>, dst: &mut Populator<FileDisk>, src_dir: u64, dst_dir: u64) -> Result<()> {
    let entries = src
        .read_dir(src_dir)
        .map_err(|e| anyhow!("{e}"))
        .with_context(|| format!("reading directory at block {src_dir}"))?;

    for entry in entries.iter().rev() {
        let ami_display = display_name(&entry.name);
        let comment = src
            .comment(entry)
            .map_err(|e| anyhow!("{e}"))
            .with_context(|| format!("reading comment of {ami_display}"))?;
        let meta = Metadata::new()
            .protection(entry.protection)
            .date(entry.date)
            .owner(entry.owner)
            .comment(&comment);

        match entry.kind {
            EntryKind::Directory => {
                let child = dst
                    .create_dir(dst_dir, &entry.name, &meta)
                    .map_err(|e| anyhow!("{e}"))
                    .with_context(|| format!("creating directory {ami_display}"))?;
                copy_dir(src, dst, entry.lba, child)?;
            }
            EntryKind::File => {
                let chain = src
                    .file_chain(entry.lba)
                    .map_err(|e| anyhow!("{e}"))
                    .with_context(|| format!("reading {ami_display}"))?;
                let mut offset = 0u64;
                let mut failure: Option<amiga_ffs::Error<DiskError>> = None;
                dst.create_file_with(dst_dir, &entry.name, &meta, |buf| match src.read_range(&chain, offset, buf) {
                    Ok(n) => {
                        offset += n as u64;
                        n
                    }
                    Err(e) => {
                        failure.get_or_insert(e);
                        0
                    }
                })
                .map_err(|e| anyhow!("{e}"))
                .with_context(|| format!("writing {ami_display}"))?;
                if let Some(e) = failure {
                    bail!("reading {ami_display}: {e}");
                }
            }
            other => bail!(
                "{ami_display}: {other:?} entries are not supported by repack (amiga-ffs's Populator has no way to create a link)"
            ),
        }
    }
    Ok(())
}
