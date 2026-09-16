//! `amirom combine` — wires up `amiga_rom::combine`: concatenates two
//! validated 512 KiB Kickstart ROM images into a 1 MiB blob for soft
//! kickers / maprom tools.
//!
//! # Argument order vs `romtool combine` — read before scripting this
//!
//! `amiga_rom::combine(first, second)` writes `first`'s bytes then
//! `second`'s, literally `[first, second].concat()` — nothing else. Its
//! doc comment records an oracle-confirmed quirk of `romtool combine`:
//! `romtool combine kick.rom ext.rom -o out.rom` actually writes
//! `ext.rom`'s bytes *first* — romtool's own CLI argument order is
//! reversed from what its argument names suggest. This CLI keeps the
//! crate's honest `first`/`second` naming instead of reproducing that
//! footgun: `amirom combine a.rom b.rom -o out.rom` writes `a.rom` then
//! `b.rom`, exactly as named. A `romtool combine kick.rom ext.rom` user
//! who wants byte-for-byte the same output must swap the arguments here:
//! `amirom combine ext.rom kick.rom -o out.rom`.
//!
//! # Checksum
//!
//! `combine` is a pure concatenation — it does not reseal either half's
//! checksum or compute one for the 1 MiB result (which isn't itself a
//! single checksum-validated ROM: `KickRom::check_size` only accepts
//! 256/512 KiB images). This command reports each half's own checksum
//! status instead of pretending otherwise; run `amirom info` on either
//! input for the full picture.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args as ClapArgs;

use crate::commands::{load_rom, KeyArg};

#[derive(ClapArgs)]
pub struct Args {
    /// First input — its bytes come first in the output. NOTE: this is
    /// not necessarily `romtool combine`'s first argument in effect;
    /// see this command's `--help` for the argument-order caveat.
    pub first: PathBuf,
    /// Second input — its bytes come second in the output.
    pub second: PathBuf,

    /// Where to write the combined 1 MiB image.
    #[arg(short, long)]
    pub out: PathBuf,

    #[command(flatten)]
    pub key: KeyArg,
}

pub fn run(args: Args) -> Result<()> {
    let key = args.key.load()?;
    let first = load_rom(&args.first, key.as_deref())?;
    let second = load_rom(&args.second, key.as_deref())?;

    let combined = amiga_rom::combine(&first, &second).map_err(|e| match e {
        amiga_rom::CombineError::InvalidInput { which, size } => {
            let (path, label) = match which {
                amiga_rom::InputSide::First => (&args.first, "first"),
                amiga_rom::InputSide::Second => (&args.second, "second"),
            };
            anyhow::anyhow!(
                "{label} input {} ({size} bytes) is not a valid 512 KiB Kickstart ROM image",
                path.display()
            )
        }
    })?;

    std::fs::write(&args.out, &combined)
        .with_context(|| format!("writing {}", args.out.display()))?;

    // See the module doc comment: combine() never touches checksums, so
    // report each half's own status rather than claiming anything about
    // the combined blob as a whole.
    let first_ok = amiga_rom::KickRom::new(&first).info().chk_sum_ok;
    let second_ok = amiga_rom::KickRom::new(&second).info().chk_sum_ok;
    println!(
        "wrote {} bytes to {}",
        combined.len(),
        args.out.display()
    );
    println!(
        "checksum status: first={} second={} (combine() does not reseal; \
         run `amirom info` on either input, or on the output's halves, for details)",
        if first_ok { "VALID" } else { "INVALID" },
        if second_ok { "VALID" } else { "INVALID" },
    );

    Ok(())
}
