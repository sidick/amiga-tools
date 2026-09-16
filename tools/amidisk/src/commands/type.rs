//! `amidisk <image> type <ami-path>` — print a file's contents to
//! stdout, verbatim. Amiga text is Latin-1-ish bytes, not UTF-8; writing
//! the raw bytes through is the only conversion that does not lie about
//! what is on the disk.

use std::io::{self, Write};
use std::path::Path;

use amiga_ffs::EntryKind;
use anyhow::{Context, Result};
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Path inside the volume, e.g. `s/startup-sequence`.
    pub ami_path: String,
}

pub fn run(image: &Path, args: Args) -> Result<()> {
    let mut vol = super::open_volume(image)?;
    let root = vol.root_lba();
    let entry = vol
        .lookup_path(root, args.ami_path.as_bytes())
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("looking up {}", args.ami_path))?
        .with_context(|| format!("{}: not found", args.ami_path))?;

    // A hard link to a file is followed; a soft link stores a path this
    // crate does not resolve, and anything else has no bytes to print.
    let file = match entry.kind {
        EntryKind::File => entry,
        EntryKind::LinkFile => vol
            .resolve_link(&entry)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .with_context(|| format!("resolving link {}", args.ami_path))?,
        EntryKind::SoftLink => anyhow::bail!(
            "{}: a soft link; resolving its path is the caller's job, not this crate's",
            args.ami_path
        ),
        other => anyhow::bail!("{}: not a file ({other:?})", args.ami_path),
    };

    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());
    let mut write_err: Option<io::Error> = None;
    vol.read_file_with(file.lba, |chunk| {
        if write_err.is_none() {
            if let Err(e) = out.write_all(chunk) {
                write_err = Some(e);
            }
        }
    })
    .map_err(|e| anyhow::anyhow!("{e}"))
    .with_context(|| format!("reading {}", args.ami_path))?;
    if let Some(e) = write_err {
        return Err(e).context("writing to stdout");
    }
    out.flush().context("flushing stdout")?;
    Ok(())
}
