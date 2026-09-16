//! `amidisk <image> blkdev` — underlying block device geometry.
//!
//! Reads the raw image file, not the filesystem on it: this works on an
//! unformatted or corrupt image, which `info` (which mounts a `Volume`)
//! cannot.

use std::path::Path;

use anyhow::{Context, Result};
use clap::Args as ClapArgs;

use crate::disk::FileDisk;

#[derive(ClapArgs)]
pub struct Args {}

/// 880K, the standard 3.5" DD floppy: 80 cylinders x 2 heads x 11
/// sectors x 512 bytes.
const DD_FLOPPY_BYTES: u64 = 901_120;
/// 1760K, the standard 3.5" HD floppy: 80 cylinders x 2 heads x 22
/// sectors x 512 bytes.
const HD_FLOPPY_BYTES: u64 = 1_802_240;

pub fn run(image: &Path, _args: Args) -> Result<()> {
    let disk = FileDisk::load(image).with_context(|| format!("reading {}", image.display()))?;
    let block_size = disk.block_size_raw() as u64;
    let size = disk.len() as u64;
    let block_count = size / block_size;

    println!("image:      {}", image.display());
    println!("size:       {size} bytes");
    println!("block size: {block_size} bytes");
    println!("blocks:     {block_count}");

    match size {
        DD_FLOPPY_BYTES => {
            println!("geometry:   3.5\" DD floppy, 80 cyls x 2 heads x 11 sectors")
        }
        HD_FLOPPY_BYTES => {
            println!("geometry:   3.5\" HD floppy, 80 cyls x 2 heads x 22 sectors")
        }
        _ => println!("geometry:   plain block device (size matches no known floppy format)"),
    }

    let leftover = size % block_size;
    if leftover != 0 {
        println!(
            "warning:    image size is not a whole number of {block_size}-byte blocks ({leftover} bytes left over)"
        );
    }
    Ok(())
}
