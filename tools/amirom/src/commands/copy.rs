//! `amirom copy` — stub. `romtool copy` copies a ROM verbatim (with an
//! optional checksum fix). This crate's version generalizes it into a
//! re-encode: byte-order conversion and Cloanto encode/decode, built on
//! `amiga_rom::Loader` and the (currently private-to-the-loader)
//! byte-order writer — normalize already exposes the read side, but
//! there's no public "re-encode to a target `ByteOrder`" or "Cloanto
//! encode" function yet, only decode. See report for the exact gap.

use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::Args as ClapArgs;

use crate::commands::KeyArg;

#[derive(ClapArgs)]
pub struct Args {
    /// Source ROM image.
    pub rom: PathBuf,
    /// Destination path.
    pub out: PathBuf,

    #[command(flatten)]
    pub key: KeyArg,

    /// Byte order to write the output in: normal, 1032, 2301, or 3210.
    #[arg(long)]
    pub byte_order: Option<String>,

    /// Cloanto-encode the output.
    #[arg(long, conflicts_with = "decode")]
    pub encode: bool,

    /// Cloanto-decode the output (same as normalize's key handling).
    #[arg(long, conflicts_with = "encode")]
    pub decode: bool,
}

pub fn run(_args: Args) -> Result<()> {
    bail!("amirom copy: not yet implemented (amiga_rom has no public byte-order re-encode or Cloanto-encode function yet)")
}
