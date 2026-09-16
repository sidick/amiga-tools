//! `amirdb <image> info` — print the RDSK/FSHD/bad-block summary.

use std::path::Path;

use amiga_rdb::{rdb_flags, CHAIN_END};
use anyhow::Result;
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {}

pub fn run(image: &Path, block_size: usize, _args: Args) -> Result<()> {
    let (rdb, mut disk) = super::open_rdb(image, block_size)?;

    println!(
        "RDSK at block {}  {} B/block  geometry {}/{}/{}  rdb blocks {}..={}",
        rdb.rdsk_block, rdb.block_bytes, rdb.cylinders, rdb.heads, rdb.sectors, rdb.rdb_blocks_lo, rdb.rdb_blocks_hi
    );
    // Only printed when the flag says the bytes mean anything: without
    // DISKID/CTRLRID these fields are uninitialised, and showing them
    // would be inventing a drive identity.
    if rdb.flags & rdb_flags::DISK_ID != 0 {
        println!("disk: {} {} rev {}", rdb.disk_vendor, rdb.disk_product, rdb.disk_revision);
    }
    if rdb.flags & rdb_flags::CTRLR_ID != 0 {
        println!(
            "controller: {} {} rev {}",
            rdb.controller_vendor, rdb.controller_product, rdb.controller_revision
        );
    }
    // The loadable filesystems the image carries — this is how a
    // partition with a dostype the ROM never heard of still mounts.
    for f in &rdb.filesystems {
        println!(
            "FSHD at block {}  {}  version {}.{}  {}",
            f.fshd_block,
            super::dostype_str(f.dos_type),
            f.version_major(),
            f.version_minor(),
            if f.seg_list_blocks == CHAIN_END {
                String::from("no LSEG chain")
            } else {
                format!("LSEG chain head block {}", f.seg_list_blocks)
            },
        );
    }
    if !rdb.bad_blocks.is_empty() {
        println!("bad blocks: {} remapped", rdb.bad_blocks.len());
    }

    // Layout validation last, so it reads as a verdict on everything
    // printed above. Both halves: `validate` covers the chains held in
    // memory, `validate_seg_lists` needs the disk back for the lazy
    // LSEG blocks. Reported, never fatal — an image whose RDB has
    // spilled into a partition is exactly the one someone is trying to
    // recover.
    let mut issues = rdb.validate();
    match rdb.validate_seg_lists(&mut disk) {
        Ok(more) => issues.extend(more),
        Err(e) => eprintln!("{}: walking LSEG chains: {e}", image.display()),
    }
    if !issues.is_empty() {
        println!("layout issues ({}):", issues.len());
        for issue in &issues {
            println!("  ! {issue}");
        }
    }

    Ok(())
}
