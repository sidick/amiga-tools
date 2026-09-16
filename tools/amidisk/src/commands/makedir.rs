//! `amidisk <image> makedir <ami-path> [--parents]` — create a directory
//! on the volume, optionally creating intermediate components too.

use std::path::Path;
use std::time::SystemTime;

use amiga_ffs::populate::datestamp_from_system_time;
use amiga_ffs::Metadata;
use anyhow::{bail, Context, Result};
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Path inside the volume to create, e.g. `s/new-dir`.
    pub ami_path: String,

    /// Create intermediate components as needed, and tolerate the final
    /// component already existing as a directory — `mkdir -p`.
    #[arg(long)]
    pub parents: bool,
}

pub fn run(image: &Path, args: Args) -> Result<()> {
    let now = datestamp_from_system_time(SystemTime::now());
    let mut mutator = super::open_mutator(image)?.clock(now);

    let components: Vec<&str> = args.ami_path.split('/').filter(|c| !c.is_empty()).collect();
    if components.is_empty() {
        bail!("{:?}: empty path", args.ami_path);
    }

    let mut cur = mutator.volume().root_lba();
    let last_index = components.len() - 1;

    for (i, comp) in components.iter().enumerate() {
        let name = ami_name_bytes(comp)?;
        let existing = mutator
            .volume()
            .lookup(cur, &name)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .with_context(|| format!("looking up {comp}"))?;

        cur = match existing {
            Some(entry) if entry.kind.is_directory() => {
                if i == last_index && !args.parents {
                    bail!("{}: already exists", args.ami_path);
                }
                entry.lba
            }
            Some(entry) => bail!("{comp}: exists and is not a directory ({:?})", entry.kind),
            None => {
                if i != last_index && !args.parents {
                    bail!("{comp}: no such directory (use --parents)");
                }
                let meta = Metadata::new().date(now);
                mutator
                    .create_dir(cur, &name, &meta)
                    .map_err(|e| anyhow::anyhow!("{e}"))
                    .with_context(|| format!("creating {comp}"))?
            }
        };
    }

    super::save_back(mutator, image)?;
    println!("{}: created", args.ami_path);
    Ok(())
}

/// A path component, given directly on the command line, as raw Latin-1
/// bytes. Unlike a host filename there is no encoding to sniff: the shell
/// handed us a `String`, and every character in it must fit a byte.
fn ami_name_bytes(s: &str) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len());
    for ch in s.chars() {
        let c = ch as u32;
        if c > 0xFF {
            bail!("{s:?}: not representable in an Amiga (Latin-1) name");
        }
        out.push(c as u8);
    }
    Ok(out)
}
