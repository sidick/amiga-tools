//! One module per subcommand, so new commands can be added without
//! disturbing the ones that already exist. This module holds what every
//! command needs: opening an image read-only or for editing, committing
//! and saving one back, and parsing the small vocabulary of sizes,
//! dostypes and geometries the CLI accepts.

pub mod add;
pub mod addimg;
pub mod adjust;
pub mod change;
pub mod create;
pub mod delete;
pub mod export;
pub mod fill;
pub mod free;
pub mod fsadd;
pub mod fsdelete;
pub mod fsflags;
pub mod fsget;
pub mod import;
pub mod info;
pub mod init;
pub mod map;
pub mod remap;
pub mod show;
pub mod validate;

use std::path::Path;

use amiga_rdb::{Rdb, RdbEditor};
use anyhow::{bail, Context, Result};

use crate::disk::FileDisk;

/// A dostype the conventional way: three printable characters and the
/// version byte as a number, e.g. `DOS\3`. Shared by partitions and
/// filesystem headers, which is the whole point — the two are matched on
/// this value.
pub fn dostype_str(v: u32) -> String {
    let b = v.to_be_bytes();
    format!("{}{}{}\\{}", b[0] as char, b[1] as char, b[2] as char, b[3])
}

/// Load a whole image into memory at `block_size`. Every command starts
/// here (directly, or through [`open_rdb`]/[`open_editor`]) except
/// `create`, which has no image to load yet.
pub fn load_disk(path: &Path, block_size: usize) -> Result<FileDisk> {
    FileDisk::load(path, block_size).with_context(|| format!("reading {}", path.display()))
}

/// Load an image and parse its RDB read-only. Every read-side command
/// (`info`, `show`, `validate`, `export`, `fsget`, ...) starts here.
pub fn open_rdb(path: &Path, block_size: usize) -> Result<(Rdb, FileDisk)> {
    let mut disk = load_disk(path, block_size)?;
    let rdb = Rdb::parse(&mut disk)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("parsing RDB in {}", path.display()))?;
    Ok((rdb, disk))
}

/// Load an image and open it for editing. Every write-side command that
/// mutates an existing table (`add`, `change`, `delete`, `adjust`,
/// `remap`, `fsadd`, `fsflags`, `fsdelete`, ...) starts here and ends at
/// [`commit_editor`]. Not yet called anywhere in this crate — every such
/// command is still a stub — so it is allowed to sit unused until one
/// lands.
#[allow(dead_code)]
pub fn open_editor(path: &Path, block_size: usize) -> Result<(RdbEditor, FileDisk)> {
    let mut disk = load_disk(path, block_size)?;
    let editor = RdbEditor::open(&mut disk)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("opening {} for editing", path.display()))?;
    Ok((editor, disk))
}

/// Commit an editor's pending edits to `disk` and write it back to
/// `path`. Every write-side command that used [`open_editor`] ends here.
/// See [`open_editor`] on why this is unused for now.
#[allow(dead_code)]
pub fn commit_editor(editor: &RdbEditor, mut disk: FileDisk, path: &Path) -> Result<amiga_rdb::CommitReport> {
    let report = editor
        .commit(&mut disk)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("committing changes to {}", path.display()))?;
    disk.save(path).with_context(|| format!("writing {}", path.display()))?;
    Ok(report)
}

/// Resolve a partition selector — a 0-based index, or a `pb_DriveName`
/// (case-insensitive) — to its index in `rdb.partitions`. Not yet
/// called anywhere in this crate — every command that would use it
/// (`change`, `delete`, `export`, `import`) is still a stub.
#[allow(dead_code)]
pub fn find_partition(rdb: &Rdb, selector: &str) -> Result<usize> {
    if let Ok(i) = selector.parse::<usize>() {
        if i < rdb.partitions.len() {
            return Ok(i);
        }
        bail!("partition index {i} out of range (0..{})", rdb.partitions.len());
    }
    rdb.partitions
        .iter()
        .position(|p| p.name.eq_ignore_ascii_case(selector))
        .with_context(|| format!("no partition named {selector:?}"))
}

/// Resolve a filesystem selector — a 0-based index, or a dostype spec
/// (see [`parse_dostype`]) — to its index in `rdb.filesystems`. See
/// [`find_partition`] on why this is unused for now.
#[allow(dead_code)]
pub fn find_filesystem(rdb: &Rdb, selector: &str) -> Result<usize> {
    if let Ok(i) = selector.parse::<usize>() {
        if i < rdb.filesystems.len() {
            return Ok(i);
        }
        bail!("filesystem index {i} out of range (0..{})", rdb.filesystems.len());
    }
    let dos_type = parse_dostype(selector)?;
    rdb.filesystems
        .iter()
        .position(|f| f.dos_type == dos_type)
        .with_context(|| format!("no filesystem for dostype {selector:?}"))
}

/// Parse a size given as a plain byte count or with a `K`/`M`/`G`/`T`
/// suffix, binary (1024-based) throughout — an optional trailing `i`
/// (`10Mi`) is accepted as a synonym for the plain form (`10M`), since
/// this crate has no 1000-based sizes to distinguish it from. A trailing
/// `B` (`10MiB`) is tolerated too.
pub fn parse_size(spec: &str) -> Result<u64> {
    let spec = spec.trim();
    let spec = spec.strip_suffix(['b', 'B']).unwrap_or(spec);
    let spec = spec.strip_suffix(['i', 'I']).unwrap_or(spec);
    let (digits, mult) = match spec.chars().last() {
        Some(c) if c.eq_ignore_ascii_case(&'k') => (&spec[..spec.len() - 1], 1024u64),
        Some(c) if c.eq_ignore_ascii_case(&'m') => (&spec[..spec.len() - 1], 1024 * 1024),
        Some(c) if c.eq_ignore_ascii_case(&'g') => (&spec[..spec.len() - 1], 1024 * 1024 * 1024),
        Some(c) if c.eq_ignore_ascii_case(&'t') => (&spec[..spec.len() - 1], 1024 * 1024 * 1024 * 1024),
        _ => (spec, 1),
    };
    let value: u64 = digits
        .trim()
        .parse()
        .with_context(|| format!("{spec:?} is not a valid size (e.g. 10M, 10Mi, 4G)"))?;
    Ok(value * mult)
}

/// Parse an amitools-rdbtool-style dostype spec into a raw `de_DosType`
/// / `fhb_DosType` value. RDB dostypes are arbitrary 32-bit values (any
/// filesystem may claim one), unlike amiga-ffs's closed `Variant` set —
/// so unlike `amidisk`'s `parse_variant`, this returns a `u32` directly.
/// Accepts:
///
/// - `ofs`/`ffs` optionally combined with `intl`/`dircache` (joined by
///   `+` or `-`, e.g. `ffs+intl+dircache`), which cover `DOS\0..DOS\5`.
/// - `DOS0`..`DOS7` (also `DOS\3`).
/// - A three-character tag plus a version digit, `PDS3`/`SFS0` style
///   (also `PFS\3`), for the many non-`DOS\x` filesystems (PFS, SFS,
///   ...) RDB has no closed list of.
/// - A raw dostype, `0x444f5303`.
pub fn parse_dostype(spec: &str) -> Result<u32> {
    let trimmed = spec.trim();

    // Raw dostype, e.g. 0x444f5303.
    if let Some(hex) = trimmed.strip_prefix("0x").or_else(|| trimmed.strip_prefix("0X")) {
        return u32::from_str_radix(hex, 16).with_context(|| format!("{trimmed:?} is not a valid hex dostype"));
    }

    let lower = trimmed.to_ascii_lowercase();

    // DOS0..DOS7 (also tolerate the literal backslash form DOS\3).
    if let Some(digit) = lower.strip_prefix("dos\\").or_else(|| lower.strip_prefix("dos")) {
        if let Ok(n) = digit.parse::<u32>() {
            if n <= 7 {
                return Ok(0x444F_5300 | n);
            }
        }
    }

    // ofs/ffs [+-] intl [+-] dircache
    if lower.starts_with("ofs") || lower.starts_with("ffs") {
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
        let n: u32 = match (base, intl, dircache) {
            ("ofs", false, false) => 0,
            ("ffs", false, false) => 1,
            ("ofs", true, false) => 2,
            ("ffs", true, false) => 3,
            ("ofs", true, true) => 4,
            ("ffs", true, true) => 5,
            (_, false, true) => bail!("{spec:?}: dircache requires intl"),
            _ => bail!("unknown dostype {spec:?}"),
        };
        return Ok(0x444F_5300 | n);
    }

    // Three-character tag plus a version digit: PDS3, SFS0, or the
    // literal-backslash spelling PFS\3.
    let compact: String = trimmed.chars().filter(|&c| c != '\\').collect();
    let bytes = compact.as_bytes();
    if bytes.len() == 4 && bytes[..3].iter().all(|b| b.is_ascii_graphic()) && bytes[3].is_ascii_digit() {
        return Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3] - b'0']));
    }

    bail!("unknown dostype {spec:?} (try ofs, ffs, ffs+intl, ffs+intl+dircache, DOS0..DOS7, PDS3-style, or 0x...)")
}

/// Parse a `C,H,S` geometry spec (cylinders, heads, sectors-per-track).
pub fn parse_geometry(spec: &str) -> Result<(u32, u32, u32)> {
    let parts: Vec<&str> = spec.split(',').map(str::trim).collect();
    let [c, h, s] = match parts.as_slice() {
        [c, h, s] => [*c, *h, *s],
        _ => bail!("{spec:?} is not a C,H,S geometry, e.g. 1023,16,63"),
    };
    Ok((
        c.parse().with_context(|| format!("{c:?} is not a valid cylinder count"))?,
        h.parse().with_context(|| format!("{h:?} is not a valid head count"))?,
        s.parse().with_context(|| format!("{s:?} is not a valid sector count"))?,
    ))
}
