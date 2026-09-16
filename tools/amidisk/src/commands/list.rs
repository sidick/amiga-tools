//! `amidisk <image> list [ami-path] [--all] [--long] [--info]` — a
//! directory listing.
//!
//! Default: the immediate children of `ami-path` (or the root). `--all`
//! recurses. `--long` adds protection flags, size, an ISO 8601
//! modification date and the comment. `--info` appends a used/free
//! blocks-and-bytes footer, from the same bitmap `info` reads.
//!
//! Entries print in the volume's own hash-table order — the order
//! AmigaDOS's own `List` shows, not sorted — because sorting behind the
//! caller's back would be a second, silent transformation of what is on
//! the disk.

use std::path::Path;

use amiga_ffs::{Entry, EntryKind, Volume};
use anyhow::{Context, Result};
use clap::Args as ClapArgs;

use super::display_name;
use crate::disk::FileDisk;

#[derive(ClapArgs)]
pub struct Args {
    /// Directory to list; defaults to the volume root.
    pub ami_path: Option<String>,

    /// Recurse into subdirectories.
    #[arg(long)]
    pub all: bool,

    /// Show protection flags, size, date and comment per entry.
    #[arg(long)]
    pub long: bool,

    /// Print a used/free blocks-and-bytes footer.
    #[arg(long)]
    pub info: bool,
}

pub fn run(image: &Path, args: Args) -> Result<()> {
    let mut vol = super::open_volume(image)?;

    let start = match &args.ami_path {
        Some(p) => {
            let root = vol.root_lba();
            let entry = vol
                .lookup_path(root, p.as_bytes())
                .map_err(|e| anyhow::anyhow!("{e}"))
                .with_context(|| format!("looking up {p}"))?
                .with_context(|| format!("{p}: not found"))?;
            if entry.kind != EntryKind::Directory {
                anyhow::bail!("{p}: not a directory ({:?})", entry.kind);
            }
            entry.lba
        }
        None => vol.root_lba(),
    };

    if args.all {
        walk(&mut vol, start, "", args.long)?;
    } else {
        let entries = vol
            .read_dir(start)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("reading directory")?;
        for entry in &entries {
            print_entry(&mut vol, entry, "", args.long)?;
        }
    }

    if args.info {
        print_footer(&mut vol)?;
    }
    Ok(())
}

fn walk(vol: &mut Volume<FileDisk>, dir_lba: u64, prefix: &str, long: bool) -> Result<()> {
    let entries = vol
        .read_dir(dir_lba)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("reading directory at block {dir_lba}"))?;
    for entry in &entries {
        print_entry(vol, entry, prefix, long)?;
        if entry.kind == EntryKind::Directory {
            let name = display_name(&entry.name);
            walk(vol, entry.lba, &format!("{prefix}{name}/"), long)?;
        }
    }
    Ok(())
}

fn print_entry(vol: &mut Volume<FileDisk>, entry: &Entry, prefix: &str, long: bool) -> Result<()> {
    let name = display_name(&entry.name);
    let suffix = match entry.kind {
        EntryKind::Directory => "/",
        EntryKind::LinkDir => "/@",
        EntryKind::File => "",
        EntryKind::SoftLink | EntryKind::LinkFile => "@",
    };

    if !long {
        println!("{prefix}{name}{suffix}");
        return Ok(());
    }

    let prot = entry.protection_bits();
    let size = if entry.kind == EntryKind::File {
        entry.byte_size.to_string()
    } else {
        String::new()
    };
    let cal = entry.date.to_calendar();
    let date = format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        cal.year, cal.month, cal.day, cal.hour, cal.minute, cal.second
    );
    let comment = vol
        .comment(entry)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("reading {name}'s comment"))?;
    let comment = display_name(&comment);
    let comment = if comment.is_empty() {
        String::new()
    } else {
        format!("  ; {comment}")
    };

    println!("{prot} {size:>10} {date}  {prefix}{name}{suffix}{comment}");
    Ok(())
}

fn print_footer(vol: &mut Volume<FileDisk>) -> Result<()> {
    let stats = super::bitmap::stats(vol)?;
    println!();
    println!(
        "{} blocks used ({} bytes), {} blocks free ({} bytes)",
        stats.used_blocks,
        stats.used_bytes(),
        stats.free_blocks,
        stats.free_bytes()
    );
    Ok(())
}
