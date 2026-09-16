//! amidisk: an amitools-xdftool-alike for Amiga ADF/HDF images, built on
//! `amiga-ffs`. This skeleton covers the read side only — `list`, `info`
//! and `read` — leaving room for `format`/`write`/`delete` to join the
//! `Command` enum later without disturbing what's here.

mod commands;
mod disk;

use std::path::PathBuf;

use amiga_ffs::Volume;
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use disk::FileDisk;

#[derive(Parser)]
#[command(name = "amidisk", about = "Inspect and extract from Amiga ADF/HDF disk images")]
struct Cli {
    /// The disk image to operate on.
    image: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Recursively list the volume's contents.
    List,
    /// Print volume-level information.
    Info,
    /// Extract one file from the volume to the host filesystem.
    Read {
        /// Path inside the volume, e.g. `s/startup-sequence`.
        amiga_path: String,
        /// Where to write it; defaults to the last path component in `.`.
        host_path: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let data = std::fs::read(&cli.image)
        .with_context(|| format!("reading {}", cli.image.display()))?;
    let disk = FileDisk::new(data);
    // `None`: trust the boot block's dostype rather than demanding a
    // particular variant.
    let mut vol = Volume::open(disk, None)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("opening {}", cli.image.display()))?;

    match cli.command {
        Command::List => commands::list::run(&mut vol),
        Command::Info => commands::info::run(&vol),
        Command::Read { amiga_path, host_path } => {
            commands::read::run(&mut vol, &amiga_path, host_path)
        }
    }
}
