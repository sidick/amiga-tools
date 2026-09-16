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
//! `LSEG` block LBAs are the one thing [`amiga_rdb::Rdb`] does not keep
//! after [`amiga_rdb::Rdb::parse`] — the chain is walked lazily, and only
//! each filesystem's chain *head* ([`amiga_rdb::FileSysHeader::seg_list_blocks`])
//! survives. [`amiga_rdb::Rdb::lseg_blocks`] re-walks the chain and hands
//! back the LBAs directly, with the same checks (ID, checksum, cycle,
//! off-disk) `load_filesystem` applies — so this command no longer needs
//! to duplicate that walk by hand.

use std::collections::BTreeMap;
use std::path::Path;

use amiga_rdb::BlockSource;
use anyhow::{bail, Context, Result};
use clap::Args as ClapArgs;

use super::open_rdb;

#[derive(ClapArgs)]
pub struct Args {}

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

    for f in &rdb.filesystems {
        let lbas = rdb
            .lseg_blocks(f, &mut disk)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("walking LSEG chain")?;
        for lba in lbas {
            kind.entry(lba).or_insert('L');
        }
    }

    let mut buf = vec![0u8; disk.block_size()];

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
