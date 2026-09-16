use std::path::PathBuf;

use anyhow::Result;
use clap::Args as ClapArgs;

use crate::commands::{load_rom, KeyArg};

#[derive(ClapArgs)]
pub struct Args {
    /// ROM image to scan for resident modules.
    pub rom: PathBuf,

    #[command(flatten)]
    pub key: KeyArg,
}

pub fn run(args: Args) -> Result<()> {
    let key = args.key.load()?;
    let normalized = load_rom(&args.rom, key.as_deref())?;
    let rom = amiga_rom::KickRom::new(&normalized);

    let mut count = 0;
    for resident in rom.scan() {
        count += 1;
        let name = String::from_utf8_lossy(resident.name);
        let id = String::from_utf8_lossy(resident.id_string);
        println!(
            "{:#08x}  pri={:<4} ver={:<3} flags={:#04x}  {name:<24} {id}",
            resident.offset, resident.priority, resident.version, resident.flags,
        );
    }
    println!("{count} resident module(s) found");

    Ok(())
}
