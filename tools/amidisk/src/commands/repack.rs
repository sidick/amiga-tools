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
//! Soft links copy straight across (`Populator::create_softlink`, same
//! per-directory pass as everything else -- a soft link's target string
//! needs no target object to exist). Hard links need their target's
//! header to already exist at its *new* LBA, which is not known until
//! the whole tree has been copied, so every hard link in the source is
//! parked during the copy and created afterwards by resolving its
//! source-side target LBA (`Entry::real_entry`) to the ami-path recorded
//! while copying, then to that path's new LBA. A target the copy never
//! reached (unreachable from the root the walk started at) is an error
//! naming the link.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use amiga_ffs::{DateStamp, EntryKind, FormatOptions, Metadata, Populator, Volume};
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

    let mut lba_map: HashMap<u64, u64> = HashMap::new();
    lba_map.insert(src_root, dst_root);
    let mut pending_hardlinks: Vec<PendingHardlink> = Vec::new();
    copy_dir(&mut src, &mut pop, src_root, dst_root, &mut lba_map, &mut pending_hardlinks)
        .with_context(|| "the source's contents did not fit the target size".to_string())?;

    for link in &pending_hardlinks {
        let dst_target = *lba_map.get(&link.src_target_lba).with_context(|| {
            format!(
                "{}: hard link's target (source block {}) was never copied (unreachable from the root)",
                link.display, link.src_target_lba
            )
        })?;
        let meta = Metadata::new().protection(link.protection).date(link.date).owner(link.owner).comment(&link.comment);
        pop.create_hardlink(link.parent_lba, &link.name, &meta, dst_target)
            .map_err(|e| anyhow!("{e}"))
            .with_context(|| format!("creating hard link {}", link.display))?;
    }

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

/// A hard link found on the source, parked until every directory/file
/// has been copied and its target might have a new LBA (see the module
/// documentation).
struct PendingHardlink {
    parent_lba: u64,
    name: Vec<u8>,
    protection: u32,
    date: DateStamp,
    owner: u32,
    comment: Vec<u8>,
    src_target_lba: u64,
    display: String,
}

/// Copy one directory's entries from `src` into `dst`, recursively, in
/// the reverse of `read_dir`'s own order. `lba_map` gains one entry per
/// directory/file copied, keyed by its *source* LBA -- what a hard
/// link's target is resolved through once the whole tree is done, since
/// `Entry::real_entry` on the source names a source LBA, not a
/// destination one.
fn copy_dir(
    src: &mut Volume<FileDisk>,
    dst: &mut Populator<FileDisk>,
    src_dir: u64,
    dst_dir: u64,
    lba_map: &mut HashMap<u64, u64>,
    pending_hardlinks: &mut Vec<PendingHardlink>,
) -> Result<()> {
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
                lba_map.insert(entry.lba, child);
                copy_dir(src, dst, entry.lba, child, lba_map, pending_hardlinks)?;
            }
            EntryKind::File => {
                let chain = src
                    .file_chain(entry.lba)
                    .map_err(|e| anyhow!("{e}"))
                    .with_context(|| format!("reading {ami_display}"))?;
                let mut offset = 0u64;
                let mut failure: Option<amiga_ffs::Error<DiskError>> = None;
                let lba = dst
                    .create_file_with(dst_dir, &entry.name, &meta, |buf| match src.read_range(&chain, offset, buf) {
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
                lba_map.insert(entry.lba, lba);
            }
            EntryKind::SoftLink => {
                let target = src
                    .read_softlink(entry.lba)
                    .map_err(|e| anyhow!("{e}"))
                    .with_context(|| format!("reading soft link {ami_display}"))?;
                dst.create_softlink(dst_dir, &entry.name, &meta, &target)
                    .map_err(|e| anyhow!("{e}"))
                    .with_context(|| format!("creating soft link {ami_display}"))?;
            }
            EntryKind::LinkFile | EntryKind::LinkDir => {
                if entry.real_entry == 0 {
                    bail!("{ami_display}: hard link with no target (real_entry is 0)");
                }
                pending_hardlinks.push(PendingHardlink {
                    parent_lba: dst_dir,
                    name: entry.name.clone(),
                    protection: entry.protection,
                    date: entry.date,
                    owner: entry.owner,
                    comment: comment.clone(),
                    src_target_lba: entry.real_entry as u64,
                    display: ami_display.clone(),
                });
            }
        }
    }
    Ok(())
}
