//! `amidisk <image> info` — volume-level facts, no traversal required.

use amiga_ffs::Volume;
use anyhow::Result;

use super::display_name;
use crate::disk::FileDisk;

pub fn run(vol: &Volume<FileDisk>) -> Result<()> {
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
