//! `amidisk <image> unpack <dest-dir> [--uaem]` — extract the whole
//! volume to a host directory tree, plus a sidecar that lets `pack`
//! rebuild an equivalent image later.
//!
//! The volume lands at `<dest-dir>/<VolumeName>/`. Two sidecars are
//! written next to that directory (not inside it):
//!
//! - `<dest-dir>/<VolumeName>.amimeta` — this crate's own metadata
//!   format, described below.
//! - `<dest-dir>/<VolumeName>.bootblock` — a raw dump of blocks 0-1
//!   (1024 bytes), written only when the boot block carries anything
//!   beyond the dostype and the (advisory, mount-recomputed) root
//!   pointer `format` always writes: a non-zero checksum or boot code.
//!
//! `--uaem` additionally writes an FS-UAE-compatible `<file>.uaem`
//! sidecar next to every extracted file and directory, matching
//! amitools' `MetaInfoFSUAE` byte for byte (this is the one place this
//! crate matches amitools' own encoding, per this repo's policy of
//! keeping compatibility only for external interop formats).
//!
//! # The `.amimeta` sidecar format (v1)
//!
//! Plain text, one directive per line, LF-terminated. Quoted fields hold
//! raw Latin-1 bytes with Rust-style escaping (`\"`, `\\`, `\xNN` for
//! anything outside 0x20..=0x7E) so a name or comment with control bytes,
//! quotes or high-bit-set characters still round-trips on one line.
//!
//! ```text
//! amimeta 1
//! volume "<name>" <dostype-hex> <iso-date>
//! dir  "<ami-path>" <protect> <iso-date> "<comment>"
//! file "<ami-path>" <protect> <iso-date> "<comment>"
//! ```
//!
//! - `amimeta 1` is the version header: the first non-blank line,
//!   mandatory.
//! - `volume` records the root's own name, its dostype as a
//!   `0x444f5303`-style hex literal (parseable by [`super::parse_variant`]
//!   and unambiguous regardless of variant naming drift), and the
//!   creation date recorded in the root block (`disk_made`).
//! - `dir`/`file` lines appear in the order [`amiga_ffs::Volume::read_dir`]
//!   itself returns entries — the same order `List`/`list` shows — so
//!   that `pack`, inserting them in the *reverse* of that order per
//!   directory, reproduces the identical hash-chain layout: a
//!   [`amiga_ffs::Populator`] inserts at the head of its slot's chain,
//!   so undoing that reversal on the way back in is what makes the
//!   repacked volume list in the same order as the original. `<ami-path>`
//!   is the full path from the volume root, `/`-separated, raw Latin-1.
//!   `<protect>` is the entry's protection bits rendered the way
//!   [`amiga_ffs::Protection`]'s `Display` does (`hsparwed`, `-` for each
//!   bit that is off).
//! - The root's own protection and comment are not recorded: the root
//!   block has no such fields to restore.

use std::fs;
use std::path::{Path, PathBuf};

use amiga_ffs::{BlockSink, BlockSource, DateStamp, EntryKind, Protection, Variant, Volume};
use anyhow::{bail, Context, Result};
use clap::Args as ClapArgs;

use super::display_name;
use crate::disk::FileDisk;

#[derive(ClapArgs)]
pub struct Args {
    /// Host directory to extract into; the volume lands at
    /// `<dest_dir>/<VolumeName>/`. Created if it does not exist.
    pub dest_dir: PathBuf,

    /// Also write an FS-UAE-compatible `.uaem` sidecar next to every
    /// extracted file and directory.
    #[arg(long)]
    pub uaem: bool,
}

pub fn run(image: &Path, args: Args) -> Result<()> {
    let mut vol = super::open_volume(image)?;
    let volume_name = vol.root().name.clone();
    let variant = vol.variant();
    let created = vol.root().disk_made;

    fs::create_dir_all(&args.dest_dir)
        .with_context(|| format!("creating {}", args.dest_dir.display()))?;
    let vol_dir = args.dest_dir.join(display_name(&volume_name));
    if vol_dir.exists() {
        bail!("{} already exists; refusing to overwrite it", vol_dir.display());
    }
    fs::create_dir_all(&vol_dir).with_context(|| format!("creating {}", vol_dir.display()))?;

    let mut entries = Vec::new();
    let root = vol.root_lba();
    extract_dir(&mut vol, root, &vol_dir, &[], &mut entries, args.uaem)?;

    let sidecar = Sidecar {
        volume_name: volume_name.clone(),
        dostype: variant.dostype(),
        created,
        entries,
    };
    let meta_path = vol_dir.with_extension("amimeta");
    write_sidecar(&meta_path, &sidecar)?;

    let boot_area = read_boot_area(vol.source_mut())?;
    if boot_is_meaningful(&boot_area) {
        let boot_path = vol_dir.with_extension("bootblock");
        fs::write(&boot_path, &boot_area).with_context(|| format!("writing {}", boot_path.display()))?;
    }

    println!(
        "{} -> {} ({} entries, {})",
        image.display(),
        vol_dir.display(),
        sidecar.entries.len(),
        meta_path.display()
    );
    Ok(())
}

/// Walk one directory, extracting every entry under `host_dir` and
/// recording it in `out`. `ami_prefix` is the path from the volume root
/// to `host_dir`, empty at the root itself.
fn extract_dir(
    vol: &mut Volume<FileDisk>,
    dir_lba: u64,
    host_dir: &Path,
    ami_prefix: &[u8],
    out: &mut Vec<MetaEntry>,
    uaem: bool,
) -> Result<()> {
    let list = vol
        .read_dir(dir_lba)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("reading directory at block {dir_lba}"))?;

    for entry in list {
        let host_name = display_name(&entry.name);
        let host_path = host_dir.join(&host_name);
        let ami_path = join_path(ami_prefix, &entry.name);
        let ami_display = display_name(&ami_path);

        let comment = vol
            .comment(&entry)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .with_context(|| format!("reading comment of {ami_display}"))?;

        match entry.kind {
            EntryKind::Directory => {
                fs::create_dir_all(&host_path)
                    .with_context(|| format!("creating {}", host_path.display()))?;
                out.push(MetaEntry {
                    path: ami_path.clone(),
                    is_dir: true,
                    protection: entry.protection,
                    date: entry.date,
                    comment: comment.clone(),
                });
                if uaem {
                    write_uaem(&host_path, entry.protection, entry.date, &comment)?;
                }
                extract_dir(vol, entry.lba, &host_path, &ami_path, out, uaem)?;
            }
            EntryKind::File => {
                let content = vol
                    .read_file(entry.lba)
                    .map_err(|e| anyhow::anyhow!("{e}"))
                    .with_context(|| format!("reading {ami_display}"))?;
                fs::write(&host_path, &content)
                    .with_context(|| format!("writing {}", host_path.display()))?;
                out.push(MetaEntry {
                    path: ami_path.clone(),
                    is_dir: false,
                    protection: entry.protection,
                    date: entry.date,
                    comment: comment.clone(),
                });
                if uaem {
                    write_uaem(&host_path, entry.protection, entry.date, &comment)?;
                }
            }
            other => bail!("{ami_display}: {other:?} entries are not supported by unpack (hard/soft links have no host representation this crate can derive)"),
        }
    }
    Ok(())
}

fn join_path(prefix: &[u8], name: &[u8]) -> Vec<u8> {
    let mut path = Vec::with_capacity(prefix.len() + 1 + name.len());
    path.extend_from_slice(prefix);
    if !path.is_empty() {
        path.push(b'/');
    }
    path.extend_from_slice(name);
    path
}

// ---------------------------------------------------------------------------
// The .amimeta sidecar
// ---------------------------------------------------------------------------

/// One recorded entry: enough for `pack` to reproduce it exactly.
pub(crate) struct MetaEntry {
    /// Full path from the volume root, `/`-separated, raw Latin-1.
    pub path: Vec<u8>,
    pub is_dir: bool,
    pub protection: u32,
    pub date: DateStamp,
    pub comment: Vec<u8>,
}

pub(crate) struct Sidecar {
    pub volume_name: Vec<u8>,
    pub dostype: u32,
    pub created: DateStamp,
    pub entries: Vec<MetaEntry>,
}

/// The path bytes of an entry's parent: everything before the last `/`,
/// or empty for a root-level entry.
pub(crate) fn parent_of(path: &[u8]) -> &[u8] {
    match path.iter().rposition(|&b| b == b'/') {
        Some(i) => &path[..i],
        None => &[],
    }
}

/// The last component of a `/`-separated path.
pub(crate) fn last_component(path: &[u8]) -> &[u8] {
    match path.iter().rposition(|&b| b == b'/') {
        Some(i) => &path[i + 1..],
        None => path,
    }
}

pub(crate) fn write_sidecar(path: &Path, sidecar: &Sidecar) -> Result<()> {
    let mut out = String::new();
    out.push_str("amimeta 1\n");
    out.push_str(&format!(
        "volume {} 0x{:08x} {}\n",
        quote_bytes(&sidecar.volume_name),
        sidecar.dostype,
        format_date(sidecar.created)
    ));
    for e in &sidecar.entries {
        out.push_str(&format!(
            "{} {} {} {} {}\n",
            if e.is_dir { "dir " } else { "file" },
            quote_bytes(&e.path),
            format_protect(e.protection),
            format_date(e.date),
            quote_bytes(&e.comment),
        ));
    }
    fs::write(path, out).with_context(|| format!("writing {}", path.display()))
}

pub(crate) fn read_sidecar(path: &Path) -> Result<Sidecar> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());

    let header = lines.next().context("empty .amimeta file")?;
    if header.trim() != "amimeta 1" {
        bail!("{}: unrecognised .amimeta header {header:?}", path.display());
    }

    let volume_line = lines
        .next()
        .with_context(|| format!("{}: missing 'volume' line", path.display()))?;
    let rest = volume_line
        .strip_prefix("volume ")
        .with_context(|| format!("{}: expected 'volume' line, got {volume_line:?}", path.display()))?;
    let (volume_name, rest) = unquote_bytes(rest)?;
    let mut parts = rest.trim_start().splitn(2, ' ');
    let dostype_hex = parts.next().unwrap_or_default();
    let dostype = u32::from_str_radix(
        dostype_hex.strip_prefix("0x").unwrap_or(dostype_hex),
        16,
    )
    .with_context(|| format!("{}: bad dostype {dostype_hex:?}", path.display()))?;
    let created = parse_date(parts.next().unwrap_or_default().trim())?;

    let mut entries = Vec::new();
    for line in lines {
        let (is_dir, rest) = if let Some(r) = line.strip_prefix("dir ") {
            (true, r)
        } else if let Some(r) = line.strip_prefix("file ") {
            (false, r)
        } else {
            bail!("{}: unrecognised line {line:?}", path.display());
        };
        let (entry_path, rest) = unquote_bytes(rest.trim_start())?;
        let rest = rest.trim_start();
        let (protect_str, rest) = rest
            .split_once(' ')
            .with_context(|| format!("{}: truncated entry line {line:?}", path.display()))?;
        let protection = parse_protect(protect_str)?;
        let rest = rest.trim_start();
        let (date_str, rest) = rest
            .split_once(' ')
            .with_context(|| format!("{}: truncated entry line {line:?}", path.display()))?;
        let date = parse_date(date_str)?;
        let (comment, _) = unquote_bytes(rest.trim_start())?;
        entries.push(MetaEntry {
            path: entry_path,
            is_dir,
            protection,
            date,
            comment,
        });
    }

    Ok(Sidecar {
        volume_name,
        dostype,
        created,
        entries,
    })
}

/// Render raw Latin-1 bytes as a double-quoted, single-line token: `"`
/// and `\` are backslash-escaped, anything outside printable ASCII
/// (0x20..=0x7E) becomes `\xNN`, so every byte round-trips through one
/// line of text.
pub(crate) fn quote_bytes(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() + 2);
    out.push('"');
    for &b in bytes {
        match b {
            b'"' => out.push_str("\\\""),
            b'\\' => out.push_str("\\\\"),
            0x20..=0x7E => out.push(b as char),
            _ => out.push_str(&format!("\\x{b:02x}")),
        }
    }
    out.push('"');
    out
}

/// The inverse of [`quote_bytes`]: `s` must start with the opening `"`.
/// Returns the decoded bytes and whatever follows the closing `"`.
pub(crate) fn unquote_bytes(s: &str) -> Result<(Vec<u8>, &str)> {
    let rest = s.strip_prefix('"').context("expected a quoted field")?;
    let bytes = rest.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => return Ok((out, &rest[i + 1..])),
            b'\\' => {
                let esc = *bytes.get(i + 1).context("dangling escape in quoted field")?;
                match esc {
                    b'"' => {
                        out.push(b'"');
                        i += 2;
                    }
                    b'\\' => {
                        out.push(b'\\');
                        i += 2;
                    }
                    b'x' => {
                        let hex = rest.get(i + 2..i + 4).context("truncated \\x escape")?;
                        out.push(u8::from_str_radix(hex, 16).context("bad \\x escape")?);
                        i += 4;
                    }
                    other => bail!("unknown escape \\{}", other as char),
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    bail!("unterminated quoted field")
}

/// An ISO-ish `YYYY-MM-DDTHH:MM:SS.TT` rendering of a `DateStamp`, where
/// `TT` is the Amiga tick (1/50 s, `00`..`49`) rather than a rounded
/// fractional second — exact, and the same field FS-UAE's `.uaem` format
/// uses for its own sub-second digits.
pub(crate) fn format_date(date: DateStamp) -> String {
    let c = date.to_calendar();
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:02}",
        c.year, c.month, c.day, c.hour, c.minute, c.second, c.tick
    )
}

pub(crate) fn parse_date(s: &str) -> Result<DateStamp> {
    let (date_part, time_part) = s.split_once('T').with_context(|| format!("{s:?} is not an ISO date"))?;
    let mut d = date_part.split('-');
    let year: i32 = d.next().context("missing year")?.parse().context("bad year")?;
    let month: u32 = d.next().context("missing month")?.parse().context("bad month")?;
    let day: u32 = d.next().context("missing day")?.parse().context("bad day")?;
    let (hms, tick_str) = time_part.split_once('.').with_context(|| format!("{s:?} is missing its tick field"))?;
    let mut t = hms.split(':');
    let hour: u32 = t.next().context("missing hour")?.parse().context("bad hour")?;
    let minute: u32 = t.next().context("missing minute")?.parse().context("bad minute")?;
    let second: u32 = t.next().context("missing second")?.parse().context("bad second")?;
    let tick: u32 = tick_str.parse().context("bad tick")?;
    let cal = amiga_ffs::CalendarDate {
        year,
        month,
        day,
        hour,
        minute,
        second,
        tick,
    };
    DateStamp::from_calendar(cal).with_context(|| format!("{s:?} is out of DateStamp's range"))
}

/// Render a protection longword as `hsparwed`, matching
/// [`Protection`]'s own `Display` (and, byte for byte, the `hsparwed`
/// string amitools' `ProtectFlags.__str__` writes into a `.uaem` file).
pub(crate) fn format_protect(bits: u32) -> String {
    Protection::from_bits(bits).to_string()
}

/// The letters this format uses, in display order, paired with their bit
/// and whether that bit's sense is inverted (a *clear* bit means the
/// letter is shown) — `r`/`w`/`e`/`d`, the four AmigaDOS has always shown
/// backwards.
const PROTECT_LETTERS: [(u8, u32, bool); 8] = [
    (b'h', amiga_ffs::meta::FIBF_HIDDEN, false),
    (b's', amiga_ffs::meta::FIBF_SCRIPT, false),
    (b'p', amiga_ffs::meta::FIBF_PURE, false),
    (b'a', amiga_ffs::meta::FIBF_ARCHIVE, false),
    (b'r', amiga_ffs::meta::FIBF_READ, true),
    (b'w', amiga_ffs::meta::FIBF_WRITE, true),
    (b'e', amiga_ffs::meta::FIBF_EXECUTE, true),
    (b'd', amiga_ffs::meta::FIBF_DELETE, true),
];

pub(crate) fn parse_protect(s: &str) -> Result<u32> {
    let bytes = s.as_bytes();
    if bytes.len() != 8 {
        bail!("{s:?} is not an 8-character hsparwed protection string");
    }
    let mut bits = 0u32;
    for (i, &(letter, mask, inverted)) in PROTECT_LETTERS.iter().enumerate() {
        match bytes[i] {
            c if c == letter => {
                if !inverted {
                    bits |= mask;
                }
            }
            b'-' => {
                if inverted {
                    bits |= mask;
                }
            }
            other => bail!(
                "{s:?}: unexpected {:?} at position {i} (expected {:?} or '-')",
                other as char,
                letter as char
            ),
        }
    }
    Ok(bits)
}

// ---------------------------------------------------------------------------
// The boot block sidecar
// ---------------------------------------------------------------------------

/// Read the 1024-byte boot area (blocks 0-1 at the usual 512-byte block
/// size; this tool's `create`/`format` never write any other size).
pub(crate) fn read_boot_area(src: &mut FileDisk) -> Result<Vec<u8>> {
    let bs = src.block_size_raw();
    let len = amiga_ffs::BOOT_AREA_LEN.max(bs);
    let blocks = len.div_ceil(bs);
    let mut area = vec![0u8; blocks * bs];
    for (i, chunk) in area.chunks_mut(bs).enumerate() {
        src.read_block(i as u64, chunk)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .with_context(|| format!("reading boot block {i}"))?;
    }
    area.truncate(len);
    Ok(area)
}

/// Whether a boot area holds anything `format` (with `boot_checksum:
/// false`, the default) would not have written itself: `format` always
/// writes the dostype and an advisory root-block pointer, so those two
/// fields are excluded and only the checksum and the boot code proper
/// are checked.
pub(crate) fn boot_is_meaningful(area: &[u8]) -> bool {
    area.get(4..8).is_some_and(|s| s.iter().any(|&b| b != 0))
        || area.get(12..).is_some_and(|s| s.iter().any(|&b| b != 0))
}

/// Recompute and store a boot area's checksum (the end-around-carry
/// algorithm, distinct from every other block's) over its current
/// contents.
pub(crate) fn recompute_boot_checksum(area: &mut [u8]) {
    let ck = amiga_ffs::bootblock_checksum(area);
    area[4..8].copy_from_slice(&ck.to_be_bytes());
}

/// Restore a captured boot area onto a freshly written image: force the
/// dostype longword to match `variant` (in case a caller's `--dos-type`
/// overrode the sidecar's own), then fix the checksum.
pub(crate) fn write_bootblock(disk: &mut FileDisk, area: &[u8], variant: Variant) -> Result<()> {
    let bs = disk.block_size_raw();
    let mut buf = area.to_vec();
    buf.resize(bs.max(amiga_ffs::BOOT_AREA_LEN), 0);
    buf[0..4].copy_from_slice(&variant.dostype().to_be_bytes());
    recompute_boot_checksum(&mut buf);
    for (i, chunk) in buf.chunks(bs).enumerate() {
        disk.write_block(i as u64, chunk).map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// The .uaem sidecar (FS-UAE interop; amitools' own encoding, matched
// exactly — see this module's documentation)
// ---------------------------------------------------------------------------

fn write_uaem(host_path: &Path, protection: u32, date: DateStamp, comment: &[u8]) -> Result<()> {
    let c = date.to_calendar();
    // amitools' MetaInfoFSUAE renders the comment as host text (its
    // FSString handles the codepage); Latin-1 -> Unicode code point is
    // the same mapping this crate's own `display_name` uses.
    let comment_text = display_name(comment);
    let line = format!(
        "{} {:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:02} {}\n",
        format_protect(protection),
        c.year,
        c.month,
        c.day,
        c.hour,
        c.minute,
        c.second,
        c.tick,
        comment_text
    );
    let mut uaem_name = host_path.as_os_str().to_os_string();
    uaem_name.push(".uaem");
    fs::write(&uaem_name, line.as_bytes())
        .with_context(|| format!("writing {}", Path::new(&uaem_name).display()))
}
