//! One module per subcommand, so new commands can be added without
//! disturbing the ones that already exist. This module holds what every
//! command needs: opening an image (read-only or for mutation), saving
//! one back, and parsing the small vocabulary of sizes and dostypes the
//! CLI accepts.

pub mod bitmap;
pub mod block;
pub mod blkdev;
pub mod boot;
pub mod comment;
pub mod create;
pub mod defrag;
pub mod delete;
pub mod format;
pub mod info;
pub mod list;
pub mod makedir;
pub mod makelink;
pub mod pack;
pub mod protect;
pub mod read;
pub mod relabel;
pub mod repack;
pub mod repair;
pub mod resize;
pub mod root;
pub mod time;
pub mod r#type;
pub mod unpack;
pub mod validate;
pub mod write;

use std::path::Path;

use amiga_ffs::{Mutator, Variant, Volume};
use anyhow::{bail, Context, Result};

use crate::disk::FileDisk;

/// Amiga filenames are Latin-1-ish bytes, not UTF-8. This is a display
/// approximation only: bytes 0x80-0xFF map to the matching Unicode code
/// points, which happens to be correct for Latin-1 but not for whatever
/// codepage a given disk's author actually typed in.
pub fn display_name(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| b as char).collect()
}

/// Load an image read-only and mount it as a [`Volume`]. `None`: trust
/// the boot block's dostype rather than demanding a particular variant.
pub fn open_volume(path: &Path) -> Result<Volume<FileDisk>> {
    let disk = FileDisk::load(path).with_context(|| format!("reading {}", path.display()))?;
    Volume::open(disk, None)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("opening {}", path.display()))
}

/// Load an image and open it for mutation. Every write-side command
/// (`makedir`, `write`, `delete`, `protect`, ...) starts here and ends
/// at [`save_back`]. Not yet called anywhere in this crate — every
/// write-side command is still a stub — so it is allowed to sit unused
/// until one lands.
#[allow(dead_code)]
pub fn open_mutator(path: &Path) -> Result<Mutator<FileDisk>> {
    let vol = open_volume(path)?;
    Mutator::open(vol)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("preparing {} for writing", path.display()))
}

/// Unwind a [`Mutator`] back down to the underlying image bytes and
/// write them to `path`: `Mutator::into_volume` -> `Volume::into_inner`
/// -> `FileDisk::save`. Every write-side command ends with this call.
/// See [`open_mutator`] on why this is unused for now.
#[allow(dead_code)]
pub fn save_back(mutator: Mutator<FileDisk>, path: &Path) -> Result<()> {
    let disk = mutator.into_volume().into_inner();
    disk.save(path).with_context(|| format!("writing {}", path.display()))
}

/// Parse a size given as a plain byte count or with a `K`/`M`/`G`
/// (binary: 1024-based) suffix, e.g. `880K`, `1760K`, `4M`. Case
/// insensitive; a trailing `B` (`880KB`) is tolerated.
pub fn parse_size(spec: &str) -> Result<u64> {
    let spec = spec.trim();
    let spec = spec.strip_suffix(['b', 'B']).unwrap_or(spec);
    let (digits, mult) = match spec.chars().last() {
        Some(c) if c.eq_ignore_ascii_case(&'k') => (&spec[..spec.len() - 1], 1024u64),
        Some(c) if c.eq_ignore_ascii_case(&'m') => (&spec[..spec.len() - 1], 1024 * 1024),
        Some(c) if c.eq_ignore_ascii_case(&'g') => (&spec[..spec.len() - 1], 1024 * 1024 * 1024),
        _ => (spec, 1),
    };
    let value: u64 = digits
        .trim()
        .parse()
        .with_context(|| format!("{spec:?} is not a valid size (e.g. 880K, 1760K, 4M)"))?;
    Ok(value * mult)
}

/// Parse an amitools-xdftool-style dostype spec into a [`Variant`]:
/// `ofs`/`ffs` optionally combined with `intl`/`dircache` (joined by
/// `+` or `-`, e.g. `ffs+intl+dircache` or `ffs-intl-dircache`),
/// `DOS0`..`DOS7`, or a raw dostype (`0x444f5303`).
pub fn parse_variant(spec: &str) -> Result<Variant> {
    let trimmed = spec.trim();

    // Raw dostype, e.g. 0x444f5303.
    if let Some(hex) = trimmed.strip_prefix("0x").or_else(|| trimmed.strip_prefix("0X")) {
        let dostype = u32::from_str_radix(hex, 16)
            .with_context(|| format!("{trimmed:?} is not a valid hex dostype"))?;
        return Variant::from_dostype(dostype)
            .with_context(|| format!("{trimmed:?} is not a DOS\\0..DOS\\7 dostype"));
    }

    // DOS0..DOS7 (also tolerate the literal backslash form DOS\3).
    let lower = trimmed.to_ascii_lowercase();
    if let Some(digit) = lower.strip_prefix("dos\\").or_else(|| lower.strip_prefix("dos")) {
        let n: u32 = digit
            .parse()
            .with_context(|| format!("{trimmed:?} is not DOS0..DOS7"))?;
        return Variant::from_dostype(amiga_ffs::DOSTYPE_MAGIC | n)
            .with_context(|| format!("{trimmed:?} is not DOS0..DOS7"));
    }

    // ofs/ffs [+-] intl [+-] dircache
    let mut parts = lower.split(['+', '-']);
    let base = parts.next().unwrap_or_default();
    let mut intl = false;
    let mut dircache = false;
    for part in parts {
        match part {
            "intl" => intl = true,
            "dircache" => dircache = true,
            "" => {}
            other => bail!("unknown dostype qualifier {other:?} in {spec:?}"),
        }
    }

    Ok(match (base, intl, dircache) {
        ("ofs", false, false) => Variant::Ofs,
        ("ffs", false, false) => Variant::Ffs,
        ("ofs", true, false) => Variant::OfsIntl,
        ("ffs", true, false) => Variant::FfsIntl,
        ("ofs", true, true) => Variant::OfsIntlDircache,
        ("ffs", true, true) => Variant::FfsIntlDircache,
        (_, false, true) => bail!("{spec:?}: dircache requires intl (amiga-ffs has no plain-fold dircache variant)"),
        _ => bail!("unknown dostype {spec:?} (try ofs, ffs, ffs+intl, ffs+intl+dircache, DOS0..DOS7, or 0x...)"),
    })
}
