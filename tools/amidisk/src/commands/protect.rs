//! `amidisk <image> protect <ami-path> <flags>` — change an entry's
//! protection bits.
//!
//! `flags` follows amitools-xdftool convention: either a full set like
//! `hsparwed` (h)old, (s)cript, (p)ure, (a)rchive, (r)ead, (w)rite,
//! (e)xecute, (d)elete, or an incremental `+flags`/`-flags` form.
//!
//! The eight letters are read as a user reads `List`'s output — letter
//! present means the operation is *permitted* — even though on disk the
//! `rwed` nibble is inverted (a set bit *denies*) while `hspa` is normal
//! sense. [`amiga_ffs::Protection`] already hides that split behind
//! `readable()`/`writable()`/... accessors and its `Display` impl, so
//! this module only has to translate letters to and from that, never
//! touch a raw bit itself.

use std::path::Path;

use amiga_ffs::meta::{
    FIBF_ARCHIVE, FIBF_DELETE, FIBF_EXECUTE, FIBF_HIDDEN, FIBF_PURE, FIBF_READ, FIBF_SCRIPT,
    FIBF_WRITE,
};
use amiga_ffs::{MetaUpdate, Protection};
use anyhow::{bail, Context, Result};
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Path inside the volume.
    pub ami_path: String,

    /// New protection bits, e.g. `hsparwed`, `+swed`, `-w`.
    #[arg(allow_hyphen_values = true)]
    pub flags: String,
}

/// One `hsparwed` letter, and the raw `FIBF_*` bit it toggles. Letter
/// order matches the `List`/`Display` convention, not the bit numbering.
const LETTERS: [(char, u32); 8] = [
    ('h', FIBF_HIDDEN),
    ('s', FIBF_SCRIPT),
    ('p', FIBF_PURE),
    ('a', FIBF_ARCHIVE),
    ('r', FIBF_READ),
    ('w', FIBF_WRITE),
    ('e', FIBF_EXECUTE),
    ('d', FIBF_DELETE),
];

/// `rwed`'s bits are active-low on disk (a set bit denies); the rest are
/// active-high. "Setting the letter" therefore means clearing the bit for
/// `rwed` and setting it for `hspa`.
fn is_inverted(bit: u32) -> bool {
    matches!(bit, FIBF_READ | FIBF_WRITE | FIBF_EXECUTE | FIBF_DELETE)
}

/// Apply "letter set" (as a user would read it) to a raw longword.
fn set_letter(bits: u32, letter_bit: u32, set: bool) -> u32 {
    let set_disk_bit = set != is_inverted(letter_bit);
    if set_disk_bit {
        bits | letter_bit
    } else {
        bits & !letter_bit
    }
}

/// Parse `spec` against the current bits: a bare `hsparwed`-subset
/// replaces every one of the eight named flags (unmentioned letters are
/// cleared); a leading `+`/`-` only changes the letters that follow.
fn apply_spec(current: u32, spec: &str) -> Result<u32> {
    let spec = spec.trim();
    if spec.is_empty() {
        bail!("protection spec must not be empty");
    }

    let (incremental, set, letters) = match spec.as_bytes()[0] {
        b'+' => (true, true, &spec[1..]),
        b'-' => (true, false, &spec[1..]),
        _ => (false, true, spec),
    };

    let mut result = if incremental {
        current
    } else {
        // A bare spec restates the whole hsparwed set: start from
        // "nothing permitted" and let the letters present grant it.
        let mut base = current;
        for (_, bit) in LETTERS {
            base = set_letter(base, bit, false);
        }
        base
    };

    for ch in letters.chars() {
        let (_, bit) = LETTERS
            .iter()
            .find(|(l, _)| *l == ch)
            .copied()
            .with_context(|| format!("{ch:?} is not a protection letter (expected one of hsparwed)"))?;
        result = set_letter(result, bit, set);
    }

    Ok(result)
}

pub fn run(image: &Path, args: Args) -> Result<()> {
    let mut mutator = super::open_mutator(image)?;
    let mut vol = mutator.volume();
    let root = vol.root_lba();
    let entry = vol
        .lookup_path(root, args.ami_path.as_bytes())
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("looking up {}", args.ami_path))?
        .with_context(|| format!("{}: not found", args.ami_path))?;

    let current = entry.protection;
    let updated = apply_spec(current, &args.flags)?;

    mutator
        .set_metadata(entry.lba, &MetaUpdate::new().protection(updated))
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("setting protection on {}", args.ami_path))?;

    super::save_back(mutator, image)?;

    println!("{}: {}", args.ami_path, Protection::from_bits(updated));
    Ok(())
}
