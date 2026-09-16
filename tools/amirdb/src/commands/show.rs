//! `amirdb <image> show` — print per-partition detail.

use std::path::Path;

use anyhow::Result;
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {}

pub fn run(image: &Path, block_size: usize, _args: Args) -> Result<()> {
    let (rdb, _disk) = super::open_rdb(image, block_size)?;

    if rdb.partitions.is_empty() {
        println!("(no partitions)");
    }

    for p in &rdb.partitions {
        println!("{} ({})", p.name, super::dostype_str(p.dos_type));
        println!("  cylinders {}..={}  ({} blocks/cyl)", p.low_cyl, p.high_cyl, p.cylinder_blocks);
        println!(
            "  lba {} +{} blocks  ({} MiB)",
            p.start_lba,
            p.block_len,
            // Saturating: both factors come off the image, and a
            // hostile `de_HighCyl` makes the product overflow — a
            // debug panic, and in release a wrapped size printed as
            // fact.
            p.block_len.saturating_mul(rdb.block_bytes as u64) / (1024 * 1024)
        );
        println!(
            "  flags: {}{}  boot priority {}",
            if p.bootable { "bootable " } else { "" },
            if p.no_automount { "no-automount " } else { "" },
            p.boot_pri
        );
        println!("  buffers {}  max transfer 0x{:08x}  mask 0x{:08x}", p.num_buffers, p.max_transfer, p.mask);
        println!();
    }

    Ok(())
}
