//! `amidisk <image> boot <show|install|clear>` — boot block inspection
//! and (de)installation.
//!
//! This is raw block surgery, not a `Volume`/`Mutator` operation: the
//! boot area (dostype, checksum, root pointer, boot code) sits in the
//! first [`BOOT_AREA_LEN`] bytes of the image regardless of whether the
//! rest of it mounts, so every action here works straight on [`FileDisk`]
//! — load, edit blocks 0 (and 1, on a 512-byte image), save.
//!
//! Boot-area layout, `DOS?` magic aside: longword 0 is the dostype
//! (`amiga_ffs::format`'s own boot writer, `OFF_BOOT_ROOT` etc.), longword
//! 1 (offset 4) the checksum, longword 2 (offset 8) the root block
//! pointer; boot *code* starts at offset 12 and runs to the end of the
//! area. `install` only ever touches that code region — the dostype and
//! root pointer are the volume's, not the installer's, to change.

use std::path::{Path, PathBuf};

use amiga_ffs::{bootblock_checksum, BlockSink, BlockSource, Variant, BOOT_AREA_LEN};
use anyhow::{bail, Context, Result};
use clap::{Args as ClapArgs, Subcommand};

use crate::disk::FileDisk;

#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    pub action: Action,
}

#[derive(Subcommand)]
pub enum Action {
    /// Print the boot block's dostype, root pointer and checksum status.
    Show,
    /// Install boot code (and a valid checksum) from a host file.
    Install {
        /// Boot code to install, e.g. a bootblock image.
        file: PathBuf,
    },
    /// Zero the boot code and checksum, leaving the volume non-bootable.
    Clear,
}

/// Offset of the checksum longword.
const OFF_CHECKSUM: usize = 4;
/// Offset of the root-block-pointer longword.
const OFF_ROOT_PTR: usize = 8;
/// Where boot code starts: after dostype, checksum and root pointer.
const OFF_CODE: usize = 12;
/// The longest boot code `install` can accept.
const CODE_MAX: usize = BOOT_AREA_LEN - OFF_CODE;

/// Read the first [`BOOT_AREA_LEN`] bytes, one block at a time so this
/// works whether the image's block size is 512 (two blocks) or already
/// at least 1024 (one).
fn read_boot_area(disk: &mut FileDisk) -> Result<Vec<u8>> {
    let bs = disk.block_size_raw();
    let nblocks = BOOT_AREA_LEN.div_ceil(bs);
    let mut area = vec![0u8; nblocks * bs];
    for i in 0..nblocks {
        disk.read_block(i as u64, &mut area[i * bs..(i + 1) * bs])
            .map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    area.truncate(BOOT_AREA_LEN);
    Ok(area)
}

/// The inverse of [`read_boot_area`]: pad back out to whole blocks and
/// write each one.
fn write_boot_area(disk: &mut FileDisk, area: &[u8]) -> Result<()> {
    let bs = disk.block_size_raw();
    let nblocks = BOOT_AREA_LEN.div_ceil(bs);
    let mut padded = area.to_vec();
    padded.resize(nblocks * bs, 0);
    for i in 0..nblocks {
        disk.write_block(i as u64, &padded[i * bs..(i + 1) * bs])
            .map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    Ok(())
}

/// The end-around-carry checksum of the whole boot area is valid
/// (`bootblock_checksum`'s own contract) exactly when it sums, longword
/// by longword with carry, to `0xFFFFFFFF` — computed here over the
/// area *as stored*, checksum longword included, unlike
/// [`bootblock_checksum`] itself, which computes what to store rather
/// than verifies it.
fn checksum_valid(area: &[u8]) -> bool {
    let mut sum: u32 = 0;
    for off in (0..area.len() & !3).step_by(4) {
        let v = u32::from_be_bytes(area[off..off + 4].try_into().unwrap_or([0; 4]));
        let (s, carry) = sum.overflowing_add(v);
        sum = s.wrapping_add(carry as u32);
    }
    sum == 0xFFFF_FFFF
}

fn hexdump(area: &[u8]) {
    for (i, chunk) in area.chunks(16).enumerate() {
        let hex: Vec<String> = chunk.iter().map(|b| format!("{b:02x}")).collect();
        let ascii: String = chunk
            .iter()
            .map(|&b| if (0x20..=0x7e).contains(&b) { b as char } else { '.' })
            .collect();
        println!("{:04x}  {:<47}  {ascii}", i * 16, hex.join(" "));
    }
}

fn show(disk: &mut FileDisk) -> Result<()> {
    let area = read_boot_area(disk)?;
    let dostype = u32::from_be_bytes(area[0..4].try_into().unwrap_or([0; 4]));
    let root_ptr = u32::from_be_bytes(area[OFF_ROOT_PTR..OFF_ROOT_PTR + 4].try_into().unwrap_or([0; 4]));
    let checksum = u32::from_be_bytes(area[OFF_CHECKSUM..OFF_CHECKSUM + 4].try_into().unwrap_or([0; 4]));

    let magic = &area[0..3];
    let variant_desc = match (magic == b"DOS", Variant::from_dostype(dostype)) {
        (true, Some(v)) => format!("{v:?}"),
        (true, None) => format!("DOS, unrecognised low byte {:#04x}", dostype & 0xFF),
        (false, _) => "not a DOS-magic boot block".to_string(),
    };

    println!("dostype: {dostype:#010x} ({variant_desc})");
    println!("root pointer: {root_ptr}");
    println!("checksum: {checksum:#010x} ({})", if checksum_valid(&area) { "valid" } else { "INVALID" });
    let code_is_zero = area[OFF_CODE..].iter().all(|&b| b == 0);
    println!("boot code: {}", if code_is_zero { "none (zeroed)" } else { "present" });
    println!();
    hexdump(&area);
    Ok(())
}

fn install(disk: &mut FileDisk, image: &Path, file: &Path) -> Result<()> {
    let code = std::fs::read(file).with_context(|| format!("reading {}", file.display()))?;
    if code.len() > CODE_MAX {
        bail!("{} is {} bytes; the boot area only has room for {CODE_MAX}", file.display(), code.len());
    }

    let mut area = read_boot_area(disk)?;
    area[OFF_CODE..].fill(0);
    area[OFF_CODE..OFF_CODE + code.len()].copy_from_slice(&code);
    let ck = bootblock_checksum(&area);
    area[OFF_CHECKSUM..OFF_CHECKSUM + 4].copy_from_slice(&ck.to_be_bytes());

    write_boot_area(disk, &area)?;
    disk.save(image).with_context(|| format!("writing {}", image.display()))?;
    println!("{}: installed {} bytes of boot code from {}", image.display(), code.len(), file.display());
    Ok(())
}

fn clear(disk: &mut FileDisk, image: &Path) -> Result<()> {
    let mut area = read_boot_area(disk)?;
    area[OFF_CODE..].fill(0);
    let ck = bootblock_checksum(&area);
    area[OFF_CHECKSUM..OFF_CHECKSUM + 4].copy_from_slice(&ck.to_be_bytes());

    write_boot_area(disk, &area)?;
    disk.save(image).with_context(|| format!("writing {}", image.display()))?;
    println!("{}: boot code cleared", image.display());
    Ok(())
}

pub fn run(image: &Path, args: Args) -> Result<()> {
    let mut disk = FileDisk::load(image).with_context(|| format!("reading {}", image.display()))?;
    match args.action {
        Action::Show => show(&mut disk),
        Action::Install { file } => install(&mut disk, image, &file),
        Action::Clear => clear(&mut disk, image),
    }
}
