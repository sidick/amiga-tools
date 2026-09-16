//! `amirom patch` — applies explicit `offset=hex-bytes` edits via
//! `amiga_rom::apply_patches`, then reseals the checksum via
//! `amiga_rom::seal_checksum` (unless `--no-seal` is given). Named
//! patches (`--patch`) are still blocked: there is no named-patch table
//! yet (see `amirom patches`), so passing `--patch` fails cleanly here
//! rather than silently doing nothing.

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::Args as ClapArgs;

use crate::commands::{load_rom, KeyArg};

#[derive(ClapArgs)]
pub struct Args {
    /// ROM image to patch.
    pub rom: PathBuf,

    /// Where to write the patched ROM image.
    #[arg(short, long)]
    pub out: PathBuf,

    #[command(flatten)]
    pub key: KeyArg,

    /// Named built-in patch to apply. Repeatable; see `amirom patches`.
    /// NOT YET IMPLEMENTED — amiga-rom has no named-patch table.
    #[arg(long = "patch")]
    pub patches: Vec<String>,

    /// Explicit edit as `offset=hex-bytes`, e.g. `0x100=4e714e71`.
    /// `offset` is decimal or `0x`-prefixed hex; `hex-bytes` is a
    /// contiguous run of hex byte pairs. Repeatable.
    #[arg(long)]
    pub set: Vec<String>,

    /// Don't reseal the checksum after patching. Off by default: a
    /// patched ROM with a stale checksum is worse than useless. Use
    /// this only to deliberately produce a checksum-INVALID image, e.g.
    /// to test checksum-failure handling elsewhere.
    #[arg(long)]
    pub no_seal: bool,
}

pub fn run(args: Args) -> Result<()> {
    if !args.patches.is_empty() {
        bail!(
            "amirom patch: --patch (named patches) is not yet implemented; amiga-rom has no \
             named-patch table (see `amirom patches`). Use --set offset=hex-bytes for explicit \
             edits instead."
        );
    }
    if args.set.is_empty() {
        bail!("amirom patch: nothing to do; pass at least one --set offset=hex-bytes");
    }

    let key = args.key.load()?;
    let mut rom = load_rom(&args.rom, key.as_deref())?;

    // Parse every --set first (index order matters for error messages
    // and matches amiga_rom::apply_patches's own by-index reporting).
    let edits: Vec<(usize, Vec<u8>)> = args
        .set
        .iter()
        .enumerate()
        .map(|(i, spec)| parse_set(spec).with_context(|| format!("--set #{i} ({spec:?})")))
        .collect::<Result<_>>()?;

    // `expected` must be a snapshot of the ROM's *current* bytes so
    // apply_patches's verify pass always succeeds for a valid offset —
    // this command applies edits unconditionally, it doesn't do its own
    // old/new mismatch checking. Copied into owned buffers (not sliced
    // from `rom`) so they can coexist with the `&mut rom` borrow below.
    let mut expecteds = Vec::with_capacity(edits.len());
    for (index, (offset, replacement)) in edits.iter().enumerate() {
        let end = offset.checked_add(replacement.len()).filter(|&end| end <= rom.len());
        let end = end.ok_or_else(|| {
            anyhow::anyhow!(
                "--set #{index} at offset {offset} length {} does not fit within the \
                 {}-byte ROM",
                replacement.len(),
                rom.len()
            )
        })?;
        expecteds.push(rom[*offset..end].to_vec());
    }

    let patch_ops: Vec<amiga_rom::PatchOp> = edits
        .iter()
        .zip(expecteds.iter())
        .map(|((offset, replacement), expected)| amiga_rom::PatchOp {
            offset: *offset,
            expected,
            replacement,
        })
        .collect();

    amiga_rom::apply_patches(&mut rom, &patch_ops)
        .map_err(|e| anyhow::anyhow!("applying patches: {e}"))?;

    println!("applied {} patch(es):", edits.len());
    for ((offset, replacement), expected) in edits.iter().zip(expecteds.iter()) {
        println!(
            "  0x{offset:06X}: {} -> {}",
            hex(expected),
            hex(replacement)
        );
    }

    let sealed = if args.no_seal {
        false
    } else {
        amiga_rom::seal_checksum(&mut rom)
            .map_err(|e| anyhow::anyhow!("resealing checksum: {e}"))?;
        true
    };

    let checksum_ok = amiga_rom::KickRom::new(&rom).info().chk_sum_ok;
    std::fs::write(&args.out, &rom).with_context(|| format!("writing {}", args.out.display()))?;

    println!(
        "wrote {} bytes to {} (checksum {}{}; see `amirom info` for details)",
        rom.len(),
        args.out.display(),
        if checksum_ok { "VALID" } else { "INVALID" },
        if sealed { ", resealed" } else { ", --no-seal: left as-is" },
    );

    Ok(())
}

/// Parses one `offset=hex-bytes` `--set` argument.
fn parse_set(spec: &str) -> Result<(usize, Vec<u8>)> {
    let (offset_str, hex_str) = spec
        .split_once('=')
        .ok_or_else(|| anyhow::anyhow!("expected offset=hex-bytes, no '=' found"))?;
    let offset =
        parse_offset(offset_str).with_context(|| format!("bad offset {offset_str:?}"))?;
    let bytes = parse_hex_bytes(hex_str).with_context(|| format!("bad hex bytes {hex_str:?}"))?;
    if bytes.is_empty() {
        bail!("no bytes given");
    }
    Ok((offset, bytes))
}

/// Decimal or `0x`/`0X`-prefixed hex offset.
fn parse_offset(s: &str) -> Result<usize> {
    match s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        Some(hex) => usize::from_str_radix(hex, 16).context("not a valid hex offset"),
        None => s.parse::<usize>().context("not a valid decimal offset"),
    }
}

/// A contiguous run of hex byte pairs, e.g. `"4e714e71"` -> `[0x4e, 0x71, 0x4e, 0x71]`.
fn parse_hex_bytes(s: &str) -> Result<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        bail!("hex byte string has an odd number of digits");
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).context("not a valid hex byte"))
        .collect()
}

/// Lowercase hex, no separators — matches the `--set` syntax so output
/// can be pasted straight back into another `--set`.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
