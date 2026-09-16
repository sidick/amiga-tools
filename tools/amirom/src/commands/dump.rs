//! `amirom dump` — hex+ASCII dump of a ROM region, annotating the
//! header/footer/magic-reset fields (per `amiga_rom`'s documented fixed
//! offsets) when they fall within the dumped range. `romtool dump`'s
//! rough equivalent, minus its `-a`/`-b` address-vs-offset toggle: this
//! always shows file offsets, since `amiga_rom::KickRom` doesn't derive
//! an in-memory address for arbitrary bytes anyway (only for the header
//! fields themselves, which is what the annotations are for).

use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::Args as ClapArgs;

use crate::commands::{fmt_hex32, fmt_rev, load_rom, KeyArg};

#[derive(ClapArgs)]
pub struct Args {
    /// ROM image to dump.
    pub rom: PathBuf,

    #[command(flatten)]
    pub key: KeyArg,

    /// Start offset within the normalized image (decimal, or `0x`-prefixed hex).
    #[arg(long, value_parser = parse_offset, default_value = "0")]
    pub offset: usize,

    /// Number of bytes to dump (decimal, or `0x`-prefixed hex). Defaults
    /// to the rest of the image.
    #[arg(long, value_parser = parse_offset)]
    pub length: Option<usize>,

    /// Bytes shown per row.
    #[arg(long, default_value_t = 16)]
    pub width: usize,
}

fn parse_offset(s: &str) -> Result<usize, String> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        usize::from_str_radix(hex, 16).map_err(|e| e.to_string())
    } else {
        s.parse().map_err(|e: std::num::ParseIntError| e.to_string())
    }
}

pub fn run(args: Args) -> Result<()> {
    let key = args.key.load()?;
    let normalized = load_rom(&args.rom, key.as_deref())?;

    if args.offset > normalized.len() {
        bail!("offset {:#x} is beyond the image ({} bytes)", args.offset, normalized.len());
    }
    let end = match args.length {
        Some(len) => args.offset.checked_add(len).filter(|&e| e <= normalized.len()).ok_or_else(
            || anyhow::anyhow!("offset+length exceeds the image ({} bytes)", normalized.len()),
        )?,
        None => normalized.len(),
    };
    if args.width == 0 {
        bail!("--width must be at least 1");
    }

    let region = &normalized[args.offset..end];
    dump_hex(region, args.offset, args.width);

    println!();
    print_annotations(&normalized, args.offset, end);

    Ok(())
}

/// `offset  hex bytes  ascii`, `width` bytes per row — the same layout
/// `amidisk block` uses, parameterized on row width instead of amidisk's
/// fixed 16 (a ROM dump benefits from wider rows more often).
fn dump_hex(buf: &[u8], base: usize, width: usize) {
    for (row, chunk) in buf.chunks(width).enumerate() {
        let off = base + row * width;
        print!("{off:08x}  ");
        for (i, b) in chunk.iter().enumerate() {
            print!("{b:02x} ");
            if i % 8 == 7 {
                print!(" ");
            }
        }
        for pad in chunk.len()..width {
            print!("   ");
            if pad % 8 == 7 {
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

/// Reports which of the fixed header (0x00..0x18), magic-reset (0xD0),
/// and footer (last 24 bytes) fields overlap `[start, end)`, with their
/// decoded values — the ROM's own semantics laid over the raw bytes
/// just printed, per `amiga_rom`'s documented offsets (`KickRom`'s
/// per-field doc comments in its source).
fn print_annotations(rom: &[u8], start: usize, end: usize) {
    let info = amiga_rom::KickRom::new(rom).info();
    let len = rom.len();
    let mut any = false;

    if start < 0x18 && end > 0x00 {
        println!("header (0x00..0x18):");
        println!("  base_addr: {}", fmt_hex32(info.base_addr));
        println!("  boot_pc:   {}", fmt_hex32(info.boot_pc));
        println!("  rom_rev:   {}", fmt_rev(info.rom_rev));
        println!("  exec_rev:  {}", fmt_rev(info.exec_rev));
        any = true;
    }
    if len >= 0xD2 && start < 0xD2 && end > 0xD0 {
        println!("magic reset opcode @0xd0: {}", if info.magic_reset_ok { "present" } else { "absent" });
        any = true;
    }
    if len >= 24 {
        let footer_start = len - 24;
        if start < len && end > footer_start {
            println!("footer ({footer_start:#x}..{len:#x}):");
            println!("  checksum: {}", fmt_hex32(info.check_sum));
            println!("  footer_ok: {}", info.footer_ok);
            any = true;
        }
    }
    if !any {
        println!("(no header/footer fields in this range)");
    }
}
