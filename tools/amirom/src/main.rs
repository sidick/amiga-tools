//! `amirom` — an amitools-`romtool`-equivalent CLI for Amiga Kickstart
//! ROM images, built entirely on the `amiga-rom` format library. This
//! crate owns no parsing logic itself: every subcommand is file I/O
//! plus argument handling around that library's API, glued together by
//! `commands::mod`'s shared helpers (`load_rom`, `KeyArg`, `fmt_hex32`,
//! `fmt_rev`).
//!
//! Naming note: `eprom-split`/`eprom-merge` are this crate's own
//! hi/lo-EPROM-byte-dump operations (were plain `split`/`merge` before
//! this CLI grew to cover romtool's full surface). `split`/`combine`
//! are romtool's module-splitting and Kickstart+Ext-ROM-concatenation
//! senses of those words — a different operation entirely. The old
//! names survive as hidden aliases so existing scripts don't break.

mod commands;

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

#[derive(Subcommand)]
enum Command {
    /// Report header/footer/checksum checks, revisions and machine hints.
    Info(commands::info::Args),
    /// Hex+ASCII dump of a ROM region, annotated with header/footer fields.
    Dump(commands::dump::Args),
    /// Compare two ROMs and report differing byte ranges.
    Diff(commands::diff::Args),
    /// List resident modules (RomTag structures) found in the image.
    Scan(commands::scan::Args),
    /// List ROMs known to a module-boundary catalog (not yet available).
    List(commands::list::Args),
    /// Check whether a ROM matches a module-boundary catalog entry.
    Query(commands::query::Args),
    /// Split a ROM into its LoadSeg()able resident modules.
    Split(commands::split::Args),
    /// Build a new ROM image from a set of modules.
    Build(commands::build::Args),
    /// List the named patches this tool knows how to apply.
    Patches(commands::patches::Args),
    /// Apply named or explicit patches to a ROM.
    Patch(commands::patch::Args),
    /// Concatenate a Kickstart and an Ext ROM into one 1 MiB image.
    Combine(commands::combine::Args),
    /// Copy a ROM, optionally re-encoding its byte order or Cloanto framing.
    Copy(commands::copy::Args),
    /// Decode/byte-swap a raw dump into a canonical ROM image.
    Normalize(commands::normalize::Args),
    /// Split a canonical image into hi/lo EPROM byte dumps.
    ///
    /// No hidden `split` alias here: that name is now the module-split
    /// command above (romtool's sense), so old scripts using the
    /// EPROM-split sense of `split` must be updated to `eprom-split`.
    EpromSplit(commands::eprom_split::Args),
    /// Merge a hi/lo EPROM byte-dump pair back into one image.
    #[command(alias = "merge", hide = true)]
    EpromMerge(commands::eprom_merge::Args),
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Info(args) => commands::info::run(args),
        Command::Dump(args) => commands::dump::run(args),
        Command::Diff(args) => commands::diff::run(args),
        Command::Scan(args) => commands::scan::run(args),
        Command::List(args) => commands::list::run(args),
        Command::Query(args) => commands::query::run(args),
        Command::Split(args) => commands::split::run(args),
        Command::Build(args) => commands::build::run(args),
        Command::Patches(args) => commands::patches::run(args),
        Command::Patch(args) => commands::patch::run(args),
        Command::Combine(args) => commands::combine::run(args),
        Command::Copy(args) => commands::copy::run(args),
        Command::Normalize(args) => commands::normalize::run(args),
        Command::EpromSplit(args) => commands::eprom_split::run(args),
        Command::EpromMerge(args) => commands::eprom_merge::run(args),
    }
}
