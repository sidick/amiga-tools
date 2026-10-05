//! `amirom split` — module extraction.
//!
//! Catalog-driven splitting (the byte-exact Remus/Romsplit sense of
//! `split`, keyed by a user-supplied catalog file) isn't implemented
//! yet: no catalog file format exists on the CLI side, and no catalog
//! data exists upstream either — see `amiga-rom`'s `PLAN.md`, milestone
//! 6 ("independent module-boundary detection").
//!
//! What *is* implemented here is a resident-scan fallback, built on
//! `amiga_rom::module_boundaries` over `KickRom::scan()`'s hits. Each
//! module's start is its own ROMTAG matchword (self-announcing, always
//! trustworthy); its end never runs past the next module's matchword,
//! since that word must stay intact for Kickstart's own boot-time scan
//! to find it. But `module_boundaries` documents that bound as *loose*,
//! not exact: real ROMs commonly have alignment padding or non-resident
//! glue code between one module's real last byte and the next module's
//! start, which this has no way to see. Extracted modules may therefore
//! carry a few trailing bytes that don't belong to them — this is
//! **not** byte-equivalent to a real catalog-driven split, and the CLI
//! says so every run so the distinction isn't lost.

use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::Args as ClapArgs;

use crate::commands::{load_rom, KeyArg};

#[derive(ClapArgs)]
pub struct Args {
    /// ROM image to split into modules.
    pub rom: PathBuf,

    #[command(flatten)]
    pub key: KeyArg,

    /// Directory to write extracted modules (and `index.txt`) into.
    #[arg(short, long)]
    pub out_dir: PathBuf,

    /// Only export modules whose name contains this substring.
    #[arg(long)]
    pub modules: Option<String>,

    /// Don't create a ROM-revision-named subdirectory under `--out-dir`.
    #[arg(long)]
    pub no_version_dir: bool,

    /// Don't write an `index.txt` alongside the extracted modules.
    #[arg(long)]
    pub no_index: bool,
}

pub fn run(args: Args) -> Result<()> {
    let key = args.key.load()?;
    let normalized = load_rom(&args.rom, key.as_deref())?;
    let rom = amiga_rom::KickRom::new(&normalized);

    let residents: Vec<_> = rom.scan().collect();
    if residents.is_empty() {
        bail!("amirom split: no resident modules found by scanning for ROMTAGs (is this a valid, byte-order-normalized Kickstart image?)");
    }
    let bounds = amiga_rom::module_boundaries(&residents, normalized.len());

    let out_dir = if args.no_version_dir {
        args.out_dir.clone()
    } else {
        let dirname = match rom.info().rom_rev {
            Some((maj, min)) => format!("{maj}.{min}"),
            None => "unknown".to_string(),
        };
        args.out_dir.join(dirname)
    };
    fs::create_dir_all(&out_dir).with_context(|| format!("creating {}", out_dir.display()))?;

    eprintln!(
        "warning: module boundaries are estimated from the resident scan, not a real \
         catalog — each module's trailing bytes may include padding or glue code \
         belonging to the next module; this is NOT a byte-exact split"
    );

    let mut index_lines = Vec::new();
    let mut written = 0usize;
    for (i, (resident, bound)) in residents.iter().zip(bounds.iter()).enumerate() {
        let name = String::from_utf8_lossy(resident.name);

        if let Some(substr) = &args.modules {
            if !name.contains(substr.as_str()) {
                continue;
            }
        }

        let data = &normalized[bound.start..bound.end_upper_bound];
        let filename = format!("{i:03}_{}.bin", sanitize_filename(&name));
        let path = out_dir.join(&filename);
        fs::write(&path, data).with_context(|| format!("writing {}", path.display()))?;
        written += 1;

        index_lines.push(format!(
            "{filename}\toffset={:#x}\tlen_upper_bound={}\tname={name}",
            bound.start,
            data.len()
        ));
    }

    if !args.no_index {
        let index_path = out_dir.join("index.txt");
        fs::write(&index_path, index_lines.join("\n") + "\n")
            .with_context(|| format!("writing {}", index_path.display()))?;
    }

    println!("wrote {written} module(s) to {}", out_dir.display());
    Ok(())
}

/// Replaces anything that isn't alphanumeric, `.`, `-` or `_` with `_`,
/// and falls back to `unnamed` for an empty (or all-replaced) name —
/// `Resident::name` is attacker/ROM-controlled bytes, not a filename.
fn sanitize_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') { c } else { '_' })
        .collect();
    if cleaned.is_empty() {
        "unnamed".to_string()
    } else {
        cleaned
    }
}
