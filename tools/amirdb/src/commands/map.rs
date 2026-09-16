//! `amirdb <image> map` — render the RDB header area
//! (`rdb_RDBBlocksLo..=rdb_RDBBlocksHi`) as a block allocation map, one
//! character per block, 64 per row with an LBA gutter — mirroring
//! amidisk's `bitmap` rendering (tools/amidisk/src/commands/bitmap.rs).
//!
//! Legend: `R`=RDSK, `P`=PART, `F`=FSHD, `L`=LSEG, `B`=BADB,
//! `.`=free/unused, `?`=unknown (a block whose header claims to be one
//! of the five chain types but which no chain this crate walked actually
//! reached — most likely a stray or orphaned block).
//!
//! `LSEG` block LBAs are the one thing [`amiga_rdb::Rdb`] does not
//! surface directly — it keeps each filesystem's chain *head*
//! ([`amiga_rdb::FileSysHeader::seg_list_blocks`]) but not the full list
//! of blocks the chain visits. So this command walks each chain itself,
//! reading raw blocks off the disk and following the `Next` pointer at
//! byte offset 16 that every chained block (`PART`/`FSHD`/`LSEG`/`BADB`)
//! shares — the same offset `amiga_rdb`'s own internal `walk_chain`
//! uses, just not one it exports. See this crate's implementation report
//! for the precise API gap this papers over.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use amiga_rdb::{be32, id, BlockSource, CHAIN_END};
use anyhow::{bail, Result};
use clap::Args as ClapArgs;

use super::open_rdb;

#[derive(ClapArgs)]
pub struct Args {}

/// Byte offset of the `Next` chain pointer shared by every `PART`,
/// `FSHD`, `LSEG` and `BADB` block. Not part of `amiga_rdb`'s public API
/// (it's `chain::NEXT` there, a private module) but stable: it's the
/// fifth longword every chained block starts with, documented in the
/// NDK layouts those structs are read from.
const CHAIN_NEXT_OFFSET: usize = 16;

pub fn run(image: &Path, block_size: usize, _args: Args) -> Result<()> {
    let (rdb, mut disk) = open_rdb(image, block_size)?;

    if rdb.rdb_blocks_lo > rdb.rdb_blocks_hi {
        bail!(
            "rdb_RDBBlocksLo ({}) is above rdb_RDBBlocksHi ({}); no header area to map",
            rdb.rdb_blocks_lo,
            rdb.rdb_blocks_hi
        );
    }
    let lo = rdb.rdb_blocks_lo as u64;
    let hi = rdb.rdb_blocks_hi as u64;

    let mut kind: BTreeMap<u64, char> = BTreeMap::new();
    kind.insert(rdb.rdsk_block, 'R');
    for p in &rdb.partitions {
        kind.entry(p.part_block).or_insert('P');
    }
    for f in &rdb.filesystems {
        kind.entry(f.fshd_block).or_insert('F');
    }
    for &b in &rdb.badb_blocks {
        kind.entry(b).or_insert('B');
    }

    let mut buf = vec![0u8; disk.block_size()];
    for f in &rdb.filesystems {
        let mut next = f.seg_list_blocks;
        let mut visited = BTreeSet::new();
        while next != CHAIN_END && visited.insert(next) {
            let lba = next as u64;
            if disk.read_block(lba, &mut buf).is_err() || be32(&buf, 0) != id::LSEG {
                break;
            }
            kind.entry(lba).or_insert('L');
            next = be32(&buf, CHAIN_NEXT_OFFSET);
        }
    }

    const PER_ROW: u64 = 64;
    let mut row_start = lo;
    while row_start <= hi {
        print!("{row_start:>7}: ");
        for col in 0..PER_ROW {
            let b = row_start + col;
            if b > hi {
                break;
            }
            let ch = match kind.get(&b) {
                Some(&c) => c,
                None => classify_unknown(&mut disk, &mut buf, b),
            };
            print!("{ch}");
        }
        println!();
        row_start += PER_ROW;
    }

    println!("R=RDSK P=PART F=FSHD L=LSEG B=BADB .=free/unused ?=unknown");
    Ok(())
}

/// A block outside every chain this crate walked: free space if it's
/// all zero, or `?` if it isn't — either an orphaned chain block (its
/// header still claims one of the five known types) or genuinely
/// unrecognized content.
fn classify_unknown(disk: &mut impl BlockSource, buf: &mut [u8], lba: u64) -> char {
    if disk.read_block(lba, buf).is_err() {
        return '?';
    }
    if buf.iter().all(|&b| b == 0) {
        '.'
    } else {
        '?'
    }
}
