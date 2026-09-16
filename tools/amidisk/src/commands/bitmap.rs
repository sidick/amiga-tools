//! `amidisk <image> bitmap [ami-path]` — allocation bitmap views.
//!
//! With no path: a compact map of the whole volume, one character per
//! block (`.` free, `#` used, `x` a block the bitmap has no opinion on —
//! the two boot blocks, or a page whose checksum failed), 64 per row with
//! an LBA gutter. With a path: the blocks that file or directory actually
//! occupies — header, extension and data blocks, in chain order.

use std::path::Path;

use amiga_ffs::{EntryKind, Volume};
use anyhow::{Context, Result};
use clap::Args as ClapArgs;

use crate::disk::FileDisk;

#[derive(ClapArgs)]
pub struct Args {
    /// Restrict the view to one file or directory's own blocks.
    pub ami_path: Option<String>,
}

/// Used/free block and byte counts, shared with `info` and `list --info`.
pub struct Stats {
    /// The volume's block size, for turning block counts into bytes.
    pub block_size: u64,
    /// Blocks the bitmap marks allocated.
    pub used_blocks: u64,
    /// Blocks the bitmap marks free.
    pub free_blocks: u64,
}

impl Stats {
    /// `used_blocks * block_size`.
    pub fn used_bytes(&self) -> u64 {
        self.used_blocks * self.block_size
    }

    /// `free_blocks * block_size`.
    pub fn free_bytes(&self) -> u64 {
        self.free_blocks * self.block_size
    }
}

/// Read the bitmap and reduce it to used/free counts.
pub fn stats(vol: &mut Volume<FileDisk>) -> Result<Stats> {
    let bm = vol
        .read_bitmap()
        .map_err(|e| anyhow::anyhow!("{e}"))
        .context("reading allocation bitmap")?;
    Ok(Stats {
        block_size: vol.block_size() as u64,
        used_blocks: bm.allocated_count(),
        free_blocks: bm.free_count(),
    })
}

pub fn run(image: &Path, args: Args) -> Result<()> {
    let mut vol = super::open_volume(image)?;

    match &args.ami_path {
        None => print_map(&mut vol),
        Some(p) => print_entry_blocks(&mut vol, p),
    }
}

fn print_map(vol: &mut Volume<FileDisk>) -> Result<()> {
    let bm = vol
        .read_bitmap()
        .map_err(|e| anyhow::anyhow!("{e}"))
        .context("reading allocation bitmap")?;
    if !bm.valid() {
        println!(
            "warning: bitmap_flag says this volume was not cleanly unmounted; bits below may be stale"
        );
    }

    let block_count = vol.block_count();
    const PER_ROW: u64 = 64;
    let mut lba = 0u64;
    while lba < block_count {
        print!("{lba:>7}: ");
        for col in 0..PER_ROW {
            let b = lba + col;
            if b >= block_count {
                break;
            }
            let ch = match bm.is_free(b) {
                Some(true) => '.',
                Some(false) => '#',
                None => 'x',
            };
            print!("{ch}");
        }
        println!();
        lba += PER_ROW;
    }

    println!(
        "{} used, {} free, {} blocks total ({} bytes/block)",
        bm.allocated_count(),
        bm.free_count(),
        block_count,
        vol.block_size()
    );
    Ok(())
}

fn print_entry_blocks(vol: &mut Volume<FileDisk>, path: &str) -> Result<()> {
    let root = vol.root_lba();
    let entry = vol
        .lookup_path(root, path.as_bytes())
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("looking up {path}"))?
        .with_context(|| format!("{path}: not found"))?;

    match entry.kind {
        EntryKind::Directory => {
            println!("header: {}", entry.lba);
            if entry.extension != 0 {
                println!("dircache: {}", entry.extension);
            }
        }
        EntryKind::File => {
            let chain = vol
                .file_chain(entry.lba)
                .map_err(|e| anyhow::anyhow!("{e}"))
                .with_context(|| format!("reading {path}'s data chain"))?;
            println!("header: {}", chain.header_lba);
            if !chain.extensions.is_empty() {
                let list: Vec<String> = chain.extensions.iter().map(u32::to_string).collect();
                println!("extensions: {}", list.join(", "));
            }
            let list: Vec<String> = chain.blocks.iter().map(u32::to_string).collect();
            println!("data: {} block(s): {}", chain.blocks.len(), list.join(", "));
        }
        other => println!("{path}: {other:?} has no data blocks of its own"),
    }
    Ok(())
}
