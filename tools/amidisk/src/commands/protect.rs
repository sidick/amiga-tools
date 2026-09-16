//! `amidisk <image> protect <ami-path> <flags>` — change an entry's
//! protection bits.
//!
//! `flags` follows amitools-xdftool convention: either a full set like
//! `hsparwed` (h)old, (s)cript, (p)ure, (a)rchive, (r)ead, (w)rite,
//! (e)xecute, (d)elete, or an incremental `+flags`/`-flags` form.

use std::path::Path;

use anyhow::Result;
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Path inside the volume.
    pub ami_path: String,

    /// New protection bits, e.g. `hsparwed`, `+swed`, `-w`.
    pub flags: String,
}

pub fn run(_image: &Path, _args: Args) -> Result<()> {
    anyhow::bail!("not yet implemented")
}
