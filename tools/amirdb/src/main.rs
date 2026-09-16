//! `amirdb`: an amitools-`rdbtool`-equivalent CLI for Amiga Rigid Disk
//! Block (RDB) partitioned images, built on `amiga-rdb`. This crate owns
//! no partitioning logic itself: every subcommand is argument handling
//! plus calls into that library's API, glued together by
//! `commands::mod`'s shared helpers (`open_rdb`, `open_editor`,
//! `commit_editor`, `parse_size`, `parse_dostype`, `parse_geometry`).
//!
//! Each subcommand is `Command::X(commands::x::Args)`; `commands::x::run`
//! owns everything past argument parsing. `create` needs no existing
//! image and writes the path directly; `init` accepts either an
//! existing image or `--create`; every other command goes through
//! `open_rdb`/`open_editor`.

mod commands;
mod disk;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

use disk::DEFAULT_BLOCK_SIZE;

#[derive(Parser)]
#[command(name = "amirdb", version, about = "Inspect and manipulate Amiga Rigid Disk Block images")]
struct Cli {
    /// The disk image to operate on. For `create`, the image to write;
    /// it must not already exist.
    image: PathBuf,

    /// Device block size the image is (or will be) in; for an existing
    /// image this must match the RDB's `rdb_BlockBytes`.
    #[arg(long = "block-size", default_value_t = DEFAULT_BLOCK_SIZE)]
    block_size: usize,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print the RDSK/FSHD/bad-block summary.
    Info(commands::info::Args),
    /// Print per-partition detail.
    Show(commands::show::Args),
    /// Create a new, blank image file (no RDB yet).
    Create(commands::create::Args),
    /// Write a fresh, empty RDB onto an image.
    Init(commands::init::Args),
    /// Grow the reserved RDB area or move the first usable cylinder.
    Adjust(commands::adjust::Args),
    /// Rewrite the disk's geometry.
    Remap(commands::remap::Args),
    /// Add a partition.
    Add(commands::add::Args),
    /// Add a partition and copy a host image into it.
    Addimg(commands::addimg::Args),
    /// Change a partition's mount parameters.
    Change(commands::change::Args),
    /// Report unclaimed cylinder ranges.
    Free(commands::free::Args),
    /// Add a partition spanning every unclaimed cylinder.
    Fill(commands::fill::Args),
    /// Remove a partition.
    Delete(commands::delete::Args),
    /// Show the block-level layout of the RDB area and partitions.
    Map(commands::map::Args),
    /// Copy a partition's raw blocks to a host file.
    Export(commands::export::Args),
    /// Copy a host file into a partition's raw blocks.
    Import(commands::import::Args),
    /// Extract a loadable filesystem driver's binary.
    Fsget(commands::fsget::Args),
    /// Add a loadable filesystem driver.
    Fsadd(commands::fsadd::Args),
    /// Set a filesystem driver's flags.
    Fsflags(commands::fsflags::Args),
    /// Remove a loadable filesystem driver.
    Fsdelete(commands::fsdelete::Args),
    /// Walk the RDB and report structural problems.
    Validate(commands::validate::Args),
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let image = &cli.image;
    let block_size = cli.block_size;

    match cli.command {
        Command::Info(args) => commands::info::run(image, block_size, args),
        Command::Show(args) => commands::show::run(image, block_size, args),
        Command::Create(args) => commands::create::run(image, block_size, args),
        Command::Init(args) => commands::init::run(image, block_size, args),
        Command::Adjust(args) => commands::adjust::run(image, block_size, args),
        Command::Remap(args) => commands::remap::run(image, block_size, args),
        Command::Add(args) => commands::add::run(image, block_size, args),
        Command::Addimg(args) => commands::addimg::run(image, block_size, args),
        Command::Change(args) => commands::change::run(image, block_size, args),
        Command::Free(args) => commands::free::run(image, block_size, args),
        Command::Fill(args) => commands::fill::run(image, block_size, args),
        Command::Delete(args) => commands::delete::run(image, block_size, args),
        Command::Map(args) => commands::map::run(image, block_size, args),
        Command::Export(args) => commands::export::run(image, block_size, args),
        Command::Import(args) => commands::import::run(image, block_size, args),
        Command::Fsget(args) => commands::fsget::run(image, block_size, args),
        Command::Fsadd(args) => commands::fsadd::run(image, block_size, args),
        Command::Fsflags(args) => commands::fsflags::run(image, block_size, args),
        Command::Fsdelete(args) => commands::fsdelete::run(image, block_size, args),
        Command::Validate(args) => commands::validate::run(image, block_size, args),
    }
}
