//! `amidisk <image> root` — root block details (show for now; setters
//! can come later).

use std::path::Path;

use amiga_ffs::{CalendarDate, RootBlock};
use anyhow::Result;
use clap::Args as ClapArgs;

use super::display_name;

#[derive(ClapArgs)]
pub struct Args {}

/// `YYYY-MM-DD HH:MM:SS.ticks` — the same format [`super::time`] parses
/// back, so `root`'s output round-trips through `time` unchanged.
pub(crate) fn format_iso(c: CalendarDate) -> String {
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:02}",
        c.year, c.month, c.day, c.hour, c.minute, c.second, c.tick
    )
}

fn describe(root: &RootBlock, hash_slots: usize) {
    println!("name: {}", display_name(&root.name));
    println!("created: {}", format_iso(root.disk_made.to_calendar()));
    println!("dir altered: {}", format_iso(root.dir_altered.to_calendar()));
    println!("disk altered: {}", format_iso(root.disk_altered.to_calendar()));
    println!("hash table: {hash_slots} slots");

    let pages: Vec<u32> = root.bitmap_pages.iter().copied().filter(|&p| p != 0).collect();
    println!(
        "bitmap: {} ({} page{}{})",
        if root.bitmap_flag == -1 { "valid" } else { "needs rebuild" },
        pages.len(),
        if pages.len() == 1 { "" } else { "s" },
        if root.bitmap_ext != 0 { ", extended" } else { "" }
    );
    if let Some(used) = root.blocks_used {
        println!("blocks used: {used}");
    }
    if let Some(fs_type) = root.fs_type {
        println!("fs type: {fs_type:#010x}");
    }
}

pub fn run(image: &Path, _args: Args) -> Result<()> {
    let vol = super::open_volume(image)?;
    println!("{}: {:?}", image.display(), vol.variant());
    describe(vol.root(), amiga_ffs::hash_table_size(vol.block_size()) as usize);
    Ok(())
}
