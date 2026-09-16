//! `amirom build` — stub. `romtool build` concatenates a set of
//! LoadSeg()able module binaries into a fresh ROM image. `amiga_rom`
//! has no builder counterpart to `split`/`combine` yet (no header/
//! footer synthesis beyond `seal_checksum`, no relocation of module
//! binaries) — real implementation needs that upstream first.

use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Where to write the built ROM image.
    #[arg(short, long)]
    pub out: PathBuf,

    /// LoadSeg()able module binary to include, in order. Repeatable.
    #[arg(long = "module")]
    pub modules: Vec<PathBuf>,

    /// Take every file in this directory as a module, in directory order.
    #[arg(long, conflicts_with = "modules")]
    pub from_dir: Option<PathBuf>,

    /// Base address of the built ROM, e.g. `0xf80000`.
    #[arg(long, value_parser = parse_hex_u32)]
    pub base_addr: Option<u32>,

    /// Size of the built ROM in KiB: 256 or 512.
    #[arg(long, default_value_t = 512)]
    pub rom_size: u32,

    /// Byte value to fill unused ROM space with.
    #[arg(long, default_value = "0x00", value_parser = parse_hex_u8)]
    pub fill: u8,
}

fn parse_hex_u32(s: &str) -> Result<u32, String> {
    let s = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(s);
    u32::from_str_radix(s, 16).map_err(|e| e.to_string())
}

fn parse_hex_u8(s: &str) -> Result<u8, String> {
    let s = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(s);
    u8::from_str_radix(s, 16).map_err(|e| e.to_string())
}

pub fn run(_args: Args) -> Result<()> {
    bail!("amirom build: needs ROM-image construction in amiga-rom (no module layout/relocation or header synthesis yet)")
}
