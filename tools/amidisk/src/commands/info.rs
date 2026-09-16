//! `amidisk <image> info` — volume-level facts, no traversal required.

use std::path::Path;

use anyhow::Result;
use clap::Args as ClapArgs;

use super::display_name;

#[derive(ClapArgs)]
pub struct Args {}

pub fn run(image: &Path, _args: Args) -> Result<()> {
    let vol = super::open_volume(image)?;

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
    Ok(())
}
