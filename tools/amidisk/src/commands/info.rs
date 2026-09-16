//! `amidisk <image> info` — volume-level facts, no traversal required.

use std::path::Path;

use anyhow::Result;
use clap::Args as ClapArgs;

use super::display_name;

#[derive(ClapArgs)]
pub struct Args {}

pub fn run(image: &Path, _args: Args) -> Result<()> {
    let mut vol = super::open_volume(image)?;

    let root = vol.root();
    let variant = vol.variant();
    println!("image:       {} blocks x {} bytes", vol.block_count(), vol.block_size());
    println!("dostype:     DOS\\{} ({:?})", variant.dostype() & 0xFF, variant);
    println!("volume name: {}", display_name(&root.name));
    println!("root block:  {}", vol.root_lba());
    println!("reserved:    {} blocks", vol.reserved());
    if let Some(used) = root.blocks_used {
        println!("blocks used: {used} (long-name volume, self-reported)");
    }

    let stats = super::bitmap::stats(&mut vol)?;
    println!(
        "blocks used: {} ({} bytes, from the allocation bitmap)",
        stats.used_blocks,
        stats.used_bytes()
    );
    println!("blocks free: {} ({} bytes)", stats.free_blocks, stats.free_bytes());
    Ok(())
}
