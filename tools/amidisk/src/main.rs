//! `amidisk`: an amitools-`xdftool`-equivalent CLI for Amiga ADF/HDF disk
//! images, built on `amiga-ffs`. This crate owns no filesystem logic
//! itself: every subcommand is argument handling plus calls into that
//! library's API, glued together by `commands::mod`'s shared helpers
//! (`open_volume`, `open_mutator`, `save_back`, `parse_size`,
//! `parse_variant`).
//!
//! Each subcommand is `Command::X(commands::x::Args)`; `commands::x::run`
//! owns everything past argument parsing. `create` and `format` need no
//! existing volume and open the image path directly; every other command
//! goes through `open_volume`/`open_mutator`.

mod commands;
mod disk;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "amidisk", version, about = "Inspect and manipulate Amiga ADF/HDF disk images")]
struct Cli {
    /// The disk image to operate on. For `create`, the image to write;
    /// it must not already exist.
    image: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List the volume's contents.
    List(commands::list::Args),
    /// Print one file's contents to stdout.
    Type(commands::r#type::Args),
    /// Print volume-level information.
    Info(commands::info::Args),
    /// Extract a file (or, recursively, a directory) to the host filesystem.
    Read(commands::read::Args),
    /// Report the underlying block device's geometry.
    Blkdev(commands::blkdev::Args),
    /// Dump one block by LBA.
    Block(commands::block::Args),
    /// Show the volume's allocation bitmap.
    Bitmap(commands::bitmap::Args),
    /// Create a new, blank disk image.
    Create(commands::create::Args),
    /// Write a fresh, empty filesystem onto an image.
    Format(commands::format::Args),
    /// Inspect, install or clear the boot block.
    Boot(commands::boot::Args),
    /// Create a directory on the volume.
    Makedir(commands::makedir::Args),
    /// Write a host file (or, recursively, a directory) into the volume.
    Write(commands::write::Args),
    /// Delete a file or (with `--all`) a directory tree.
    Delete(commands::delete::Args),
    /// Change an entry's protection bits.
    Protect(commands::protect::Args),
    /// Change an entry's comment.
    Comment(commands::comment::Args),
    /// Change an entry's timestamp.
    Time(commands::time::Args),
    /// Rename the volume itself.
    Relabel(commands::relabel::Args),
    /// Show root block details.
    Root(commands::root::Args),
    /// Extract the whole volume to a host directory tree.
    Unpack(commands::unpack::Args),
    /// Populate the volume from a host directory tree.
    Pack(commands::pack::Args),
    /// Copy every entry from another image into this one.
    Repack(commands::repack::Args),
    /// Walk the volume and report structural problems.
    Validate(commands::validate::Args),
    /// Rebuild the bitmap and (optionally) sever broken chains.
    Repair(commands::repair::Args),
    /// Defragment file data and (optionally) directory headers.
    Defrag(commands::defrag::Args),
    /// Grow or shrink the volume in place.
    Resize(commands::resize::Args),
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let image = &cli.image;

    match cli.command {
        Command::List(args) => commands::list::run(image, args),
        Command::Type(args) => commands::r#type::run(image, args),
        Command::Info(args) => commands::info::run(image, args),
        Command::Read(args) => commands::read::run(image, args),
        Command::Blkdev(args) => commands::blkdev::run(image, args),
        Command::Block(args) => commands::block::run(image, args),
        Command::Bitmap(args) => commands::bitmap::run(image, args),
        Command::Create(args) => commands::create::run(image, args),
        Command::Format(args) => commands::format::run(image, args),
        Command::Boot(args) => commands::boot::run(image, args),
        Command::Makedir(args) => commands::makedir::run(image, args),
        Command::Write(args) => commands::write::run(image, args),
        Command::Delete(args) => commands::delete::run(image, args),
        Command::Protect(args) => commands::protect::run(image, args),
        Command::Comment(args) => commands::comment::run(image, args),
        Command::Time(args) => commands::time::run(image, args),
        Command::Relabel(args) => commands::relabel::run(image, args),
        Command::Root(args) => commands::root::run(image, args),
        Command::Unpack(args) => commands::unpack::run(image, args),
        Command::Pack(args) => commands::pack::run(image, args),
        Command::Repack(args) => commands::repack::run(image, args),
        Command::Validate(args) => commands::validate::run(image, args),
        Command::Repair(args) => commands::repair::run(image, args),
        Command::Defrag(args) => commands::defrag::run(image, args),
        Command::Resize(args) => commands::resize::run(image, args),
    }
}
