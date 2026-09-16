//! `amirdb` — inspect Amiga Rigid Disk Block (RDB) partitioned images.
//!
//! An amitools `rdbtool`-alike built on the `amiga-rdb` library. This
//! skeleton implements the read-only commands (`info`, `show`); mutating
//! subcommands (`init`, `add`, `free`, ...) belong as further variants of
//! [`Command`], each backed by `amiga_rdb::RdbBuilder` / `RdbEditor`.

use amiga_rdb::{rdb_flags, Rdb, SeekBlockSource, CHAIN_END};
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::fs::File;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "amirdb", about = "Inspect Amiga Rigid Disk Block images")]
struct Cli {
    /// Path to the disk image.
    image: PathBuf,

    /// Device block size the image was taken from; must match the RDB's
    /// `rdb_BlockBytes`.
    #[arg(long, default_value_t = 512)]
    block_size: usize,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print the RDSK/FSHD/bad-block summary.
    Info,
    /// Print per-partition detail.
    Show,
}

/// A dostype the conventional way: three printable characters and the
/// version byte as a number, e.g. `DOS\3`. Shared by partitions and
/// filesystem headers, which is the whole point — the two are matched on
/// this value.
fn dostype(v: u32) -> String {
    let b = v.to_be_bytes();
    format!("{}{}{}\\{}", b[0] as char, b[1] as char, b[2] as char, b[3])
}

fn open_rdb(cli: &Cli) -> Result<(Rdb, SeekBlockSource<File>)> {
    let file = File::open(&cli.image)
        .with_context(|| format!("opening {}", cli.image.display()))?;
    let mut disk = SeekBlockSource::with_block_size(file, cli.block_size)
        .with_context(|| format!("statting {}", cli.image.display()))?;
    let rdb = Rdb::parse(&mut disk)
        .with_context(|| format!("parsing RDB in {}", cli.image.display()))?;
    Ok((rdb, disk))
}

fn cmd_info(cli: &Cli) -> Result<()> {
    let (rdb, mut disk) = open_rdb(cli)?;

    println!(
        "RDSK at block {}  {} B/block  geometry {}/{}/{}  rdb blocks {}..={}",
        rdb.rdsk_block,
        rdb.block_bytes,
        rdb.cylinders,
        rdb.heads,
        rdb.sectors,
        rdb.rdb_blocks_lo,
        rdb.rdb_blocks_hi
    );
    // Only printed when the flag says the bytes mean anything: without
    // DISKID/CTRLRID these fields are uninitialised, and showing them
    // would be inventing a drive identity.
    if rdb.flags & rdb_flags::DISK_ID != 0 {
        println!(
            "disk: {} {} rev {}",
            rdb.disk_vendor, rdb.disk_product, rdb.disk_revision
        );
    }
    if rdb.flags & rdb_flags::CTRLR_ID != 0 {
        println!(
            "controller: {} {} rev {}",
            rdb.controller_vendor, rdb.controller_product, rdb.controller_revision
        );
    }
    // The loadable filesystems the image carries — this is how a
    // partition with a dostype the ROM never heard of still mounts.
    for f in &rdb.filesystems {
        println!(
            "FSHD at block {}  {}  version {}.{}  {}",
            f.fshd_block,
            dostype(f.dos_type),
            f.version_major(),
            f.version_minor(),
            if f.seg_list_blocks == CHAIN_END {
                String::from("no LSEG chain")
            } else {
                format!("LSEG chain head block {}", f.seg_list_blocks)
            },
        );
    }
    if !rdb.bad_blocks.is_empty() {
        println!("bad blocks: {} remapped", rdb.bad_blocks.len());
    }

    // Layout validation last, so it reads as a verdict on everything
    // printed above. Both halves: `validate` covers the chains held in
    // memory, `validate_seg_lists` needs the disk back for the lazy
    // LSEG blocks. Reported, never fatal — an image whose RDB has
    // spilled into a partition is exactly the one someone is trying to
    // recover.
    let mut issues = rdb.validate();
    match rdb.validate_seg_lists(&mut disk) {
        Ok(more) => issues.extend(more),
        Err(e) => eprintln!("{}: walking LSEG chains: {e}", cli.image.display()),
    }
    if !issues.is_empty() {
        println!("layout issues ({}):", issues.len());
        for issue in &issues {
            println!("  ! {issue}");
        }
    }

    Ok(())
}

fn cmd_show(cli: &Cli) -> Result<()> {
    let (rdb, _disk) = open_rdb(cli)?;

    for p in &rdb.partitions {
        println!("{} ({})", p.name, dostype(p.dos_type));
        println!(
            "  cylinders {}..={}  ({} blocks/cyl)",
            p.low_cyl, p.high_cyl, p.cylinder_blocks
        );
        println!(
            "  lba {} +{} blocks  ({} MiB)",
            p.start_lba,
            p.block_len,
            // Saturating: both factors come off the image, and a
            // hostile `de_HighCyl` makes the product overflow — a
            // debug panic, and in release a wrapped size printed as
            // fact.
            p.block_len.saturating_mul(rdb.block_bytes as u64) / (1024 * 1024)
        );
        println!(
            "  flags: {}{}  boot priority {}",
            if p.bootable { "bootable " } else { "" },
            if p.no_automount { "no-automount " } else { "" },
            p.boot_pri
        );
        println!(
            "  buffers {}  max transfer 0x{:08x}  mask 0x{:08x}",
            p.num_buffers, p.max_transfer, p.mask
        );
        println!();
    }

    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Info => cmd_info(&cli),
        Command::Show => cmd_show(&cli),
    }
}
