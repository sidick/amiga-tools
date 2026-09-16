use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args as ClapArgs;

use crate::KeyArg;

#[derive(ClapArgs)]
pub struct Args {
    /// ROM image to inspect (any recognized byte order or Cloanto encoding).
    pub rom: PathBuf,

    #[command(flatten)]
    pub key: KeyArg,
}

pub fn run(args: Args) -> Result<()> {
    let key = args.key.load()?;
    let raw =
        std::fs::read(&args.rom).with_context(|| format!("reading {}", args.rom.display()))?;
    let encoding = amiga_rom::Loader::detect(&raw);
    let normalized = amiga_rom::Loader::normalize(&raw, key.as_deref())
        .with_context(|| format!("normalizing {}", args.rom.display()))?;

    let rom = amiga_rom::KickRom::new(&normalized);
    let info = rom.info();
    let hints = rom.machine_hints();

    println!("file:              {}", args.rom.display());
    println!("detected encoding: {}", describe_encoding(encoding));
    println!("normalized size:   {} bytes", normalized.len());
    println!("is_kick_rom:       {}", info.is_kick);
    println!();
    println!("size_ok:           {}", info.size_ok);
    println!("header_ok:         {}", info.header_ok);
    println!("footer_ok:         {}", info.footer_ok);
    println!("size_field_ok:     {}", info.size_field_ok);
    println!("checksum_ok:       {}", info.chk_sum_ok);
    println!("kickety_split_ok:  {}", info.kickety_split_ok);
    println!("doubled_ok:        {}", info.doubled_ok);
    println!("magic_reset_ok:    {}", info.magic_reset_ok);
    println!();
    println!("checksum:          {}", fmt_hex32(info.check_sum));
    println!("base_addr:         {}", fmt_hex32(info.base_addr));
    println!("boot_pc:           {}", fmt_hex32(info.boot_pc));
    println!("rom_rev:           {}", fmt_rev(info.rom_rev));
    println!("exec_rev:          {}", fmt_rev(info.exec_rev));
    println!();
    println!("machine hints:");
    println!(
        "  named_machine:   {}",
        hints
            .named_machine
            .map(|n| String::from_utf8_lossy(n).into_owned())
            .unwrap_or_else(|| "-".into())
    );
    println!("  has_pcmcia:      {}", hints.has_pcmcia);
    println!("  has_ncr_scsi:    {}", hints.has_ncr_scsi);
    println!("  is_aros:         {}", hints.is_aros);
    println!(
        "  target_platform: {}",
        hints
            .target_platform
            .map(|p| String::from_utf8_lossy(p).into_owned())
            .unwrap_or_else(|| "-".into())
    );

    Ok(())
}

fn describe_encoding(encoding: Option<amiga_rom::RomEncoding>) -> String {
    use amiga_rom::{ByteOrder, RomEncoding};
    match encoding {
        None => "unrecognized".to_string(),
        Some(RomEncoding::CloantoEncoded) => "Cloanto/Amiga Forever encoded".to_string(),
        Some(RomEncoding::Raw(ByteOrder::Normal)) => "raw, normal byte order".to_string(),
        Some(RomEncoding::Raw(ByteOrder::Order1032)) => "raw, 1032 byte order".to_string(),
        Some(RomEncoding::Raw(ByteOrder::Order2301)) => "raw, 2301 byte order".to_string(),
        Some(RomEncoding::Raw(ByteOrder::Order3210)) => "raw, 3210 byte order".to_string(),
    }
}

fn fmt_hex32(v: Option<u32>) -> String {
    match v {
        Some(v) => format!("0x{v:08X}"),
        None => "-".to_string(),
    }
}

fn fmt_rev(v: Option<(u16, u16)>) -> String {
    match v {
        Some((maj, min)) => format!("{maj}.{min}"),
        None => "-".to_string(),
    }
}
