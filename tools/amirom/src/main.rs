//! `amirom` — an amitools-`romtool`-equivalent CLI for Amiga Kickstart
//! ROM images, built entirely on the `amiga-rom` format library. This
//! crate owns no parsing logic itself: every subcommand is file I/O
//! plus argument handling around that library's API.

mod cmd;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "amirom",
    version,
    about = "Inspect and manipulate Amiga Kickstart ROM images"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

/// New subcommands go here; each delegates to its own `cmd::<name>::run`.
#[derive(Subcommand)]
enum Command {
    /// Report header/footer/checksum checks, revisions and machine hints.
    Info(cmd::info::Args),
    /// List resident modules (RomTag structures) found in the image.
    Scan(cmd::scan::Args),
    /// Decode/byte-swap a raw dump into a canonical ROM image.
    Normalize(cmd::normalize::Args),
    /// Split a canonical image into hi/lo EPROM byte dumps.
    Split(cmd::split::Args),
    /// Merge a hi/lo EPROM byte-dump pair back into one image.
    Merge(cmd::merge::Args),
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Info(args) => cmd::info::run(args),
        Command::Scan(args) => cmd::scan::run(args),
        Command::Normalize(args) => cmd::normalize::run(args),
        Command::Split(args) => cmd::split::run(args),
        Command::Merge(args) => cmd::merge::run(args),
    }
}

/// Shared by every subcommand that accepts an optional Cloanto `rom.key`.
#[derive(clap::Args)]
pub struct KeyArg {
    /// Cloanto/Amiga Forever `rom.key` file, required only if the input is
    /// Cloanto-encoded.
    #[arg(short, long)]
    pub key: Option<PathBuf>,
}

impl KeyArg {
    pub fn load(&self) -> Result<Option<Vec<u8>>> {
        match &self.key {
            Some(path) => {
                let bytes = std::fs::read(path).with_context_path(path, "reading rom.key")?;
                Ok(Some(bytes))
            }
            None => Ok(None),
        }
    }
}

/// `anyhow::Context::with_context` with the path baked into the message,
/// since every I/O error in this CLI needs it and `Context::context`
/// alone can't format a `Path` without an extra closure at each call site.
trait WithContextPath<T> {
    fn with_context_path(self, path: &std::path::Path, what: &str) -> Result<T>;
}

impl<T> WithContextPath<T> for std::io::Result<T> {
    fn with_context_path(self, path: &std::path::Path, what: &str) -> Result<T> {
        use anyhow::Context;
        self.with_context(|| format!("{what}: {}", path.display()))
    }
}
