//! `amidisk <image> block <lba>` — hex+ASCII dump of one raw block, plus
//! a best-effort guess at what kind of FFS block it is.
//!
//! Reads the raw image directly rather than mounting a `Volume`, so it
//! works on any block of any image, mountable or not — which is the
//! point of a low-level inspection command.

use std::path::Path;

use amiga_ffs::layout::{tail, TL_SECONDARY_TYPE};
use amiga_ffs::{
    be32, BlockSource, ST_FILE, ST_LINKDIR, ST_LINKFILE, ST_ROOT, ST_SOFTLINK, ST_USERDIR,
    T_COMMENT, T_DATA, T_DIRCACHE, T_HEADER, T_LIST,
};
use anyhow::{Context, Result};
use clap::Args as ClapArgs;

use crate::disk::{DiskError, FileDisk};

#[derive(ClapArgs)]
pub struct Args {
    /// Block number to dump.
    pub lba: u64,
}

pub fn run(image: &Path, args: Args) -> Result<()> {
    let mut disk = FileDisk::load(image).with_context(|| format!("reading {}", image.display()))?;
    let bs = disk.block_size_raw();
    let mut buf = vec![0u8; bs];
    disk.read_block(args.lba, &mut buf).map_err(|DiskError(lba)| {
        anyhow::anyhow!("block {lba} is outside the image ({} blocks)", disk.len() / bs)
    })?;

    println!("block {} ({bs} bytes):", args.lba);
    dump_hex(&buf);

    if let Some(desc) = interpret(&buf) {
        println!();
        println!("interpretation: {desc}");
    }
    Ok(())
}

/// 16 bytes/row: offset, hex, ASCII — the conventional layout.
fn dump_hex(buf: &[u8]) {
    for (row, chunk) in buf.chunks(16).enumerate() {
        let off = row * 16;
        print!("{off:06x}  ");
        for (i, b) in chunk.iter().enumerate() {
            print!("{b:02x} ");
            if i == 7 {
                print!(" ");
            }
        }
        for pad in chunk.len()..16 {
            print!("   ");
            if pad == 7 {
                print!(" ");
            }
        }
        print!(" ");
        for &b in chunk {
            let c = if (0x20..0x7f).contains(&b) { b as char } else { '.' };
            print!("{c}");
        }
        println!();
    }
}

/// Guess what this block is from its type and (where it has one)
/// secondary-type longwords. `None` for block kinds with no type
/// longword to recognise — FFS data and bitmap blocks look identical to
/// arbitrary bytes without walking a chain to them.
fn interpret(buf: &[u8]) -> Option<String> {
    let bs = buf.len();
    if bs < 4 {
        return None;
    }
    let ty = be32(buf, 0);
    let st = if bs >= tail(bs, TL_SECONDARY_TYPE) + 4 {
        be32(buf, tail(bs, TL_SECONDARY_TYPE)) as i32
    } else {
        0
    };
    match (ty, st) {
        (T_HEADER, ST_ROOT) => Some("root block".to_string()),
        (T_HEADER, ST_USERDIR) => Some("directory header".to_string()),
        (T_HEADER, ST_FILE) => Some("file header".to_string()),
        (T_HEADER, ST_SOFTLINK) => Some("soft link".to_string()),
        (T_HEADER, ST_LINKDIR) => Some("hard link to a directory".to_string()),
        (T_HEADER, ST_LINKFILE) => Some("hard link to a file".to_string()),
        (T_LIST, _) => Some("file extension (data pointer list) block".to_string()),
        (T_DATA, _) => Some("OFS data block".to_string()),
        (T_DIRCACHE, _) => Some("directory cache block".to_string()),
        (T_COMMENT, _) => Some("overflow comment block".to_string()),
        _ => None,
    }
}
