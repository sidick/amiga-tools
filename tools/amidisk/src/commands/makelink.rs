//! `amidisk <image> makelink <link_path> <target> [--soft]` — create a
//! link on the volume.
//!
//! Hard link (default): `<target>` is an existing ami-path, resolved to
//! its header block via `Mutator::lookup_path`; `Mutator::create_hardlink`
//! then refuses it unless it is a plain file or directory (never another
//! link — see that method's own documentation for why).
//!
//! `--soft`: `<target>` is stored verbatim as the link's path, with no
//! existence check at all — a soft link may dangle, name a path on
//! another volume, or use `Assign`-relative syntax this crate has never
//! heard of, exactly like AmigaOS's own soft links.

use std::path::Path;
use std::time::SystemTime;

use amiga_ffs::populate::datestamp_from_system_time;
use amiga_ffs::Metadata;
use anyhow::{Context, Result};
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Path inside the volume for the new link.
    pub link_path: String,

    /// For a hard link (default): an existing ami-path to link to. For
    /// `--soft`: the target path string, stored verbatim.
    pub target: String,

    /// Create a soft link storing `target` as a literal path, instead of
    /// a hard link to an existing entry.
    #[arg(long)]
    pub soft: bool,
}

pub fn run(image: &Path, args: Args) -> Result<()> {
    let now = datestamp_from_system_time(SystemTime::now());
    let mut mutator = super::open_mutator(image)?.clock(now);

    let link_path = args.link_path.trim_matches('/');
    let (parent_path, name_str) = match link_path.rsplit_once('/') {
        Some((p, n)) => (p, n),
        None => ("", link_path),
    };
    let name = ami_name_bytes(name_str)?;

    let root = mutator.volume().root_lba();
    let parent_lba = if parent_path.is_empty() {
        root
    } else {
        mutator
            .volume()
            .lookup_path(root, parent_path.as_bytes())
            .map_err(|e| anyhow::anyhow!("{e}"))
            .with_context(|| format!("looking up {parent_path}"))?
            .with_context(|| format!("{parent_path}: not found"))?
            .lba
    };

    let meta = Metadata::new().date(now);

    if args.soft {
        let target_bytes = ami_name_bytes(&args.target)?;
        mutator
            .create_softlink(parent_lba, &name, &meta, &target_bytes)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .with_context(|| format!("creating soft link {}", args.link_path))?;
    } else {
        let target_entry = mutator
            .volume()
            .lookup_path(root, args.target.as_bytes())
            .map_err(|e| anyhow::anyhow!("{e}"))
            .with_context(|| format!("looking up target {}", args.target))?
            .with_context(|| format!("{}: not found", args.target))?;

        mutator
            .create_hardlink(parent_lba, &name, &meta, target_entry.lba)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .with_context(|| format!("creating hard link {} -> {}", args.link_path, args.target))?;
    }

    super::save_back(mutator, image)?;
    let kind = if args.soft { "soft" } else { "hard" };
    println!("{} -> {} ({kind} link)", args.link_path, args.target);
    Ok(())
}

/// A path or name component, given directly on the command line, as raw
/// Latin-1 bytes. Unlike a host filename there is no encoding to sniff:
/// the shell handed us a `String`, and every character in it must fit a
/// byte. Same helper as `makedir`'s and `write`'s (each module keeps its
/// own copy rather than sharing one, matching this crate's existing
/// convention).
fn ami_name_bytes(s: &str) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len());
    for ch in s.chars() {
        let c = ch as u32;
        if c > 0xFF {
            anyhow::bail!("{s:?}: not representable in an Amiga (Latin-1) name");
        }
        out.push(c as u8);
    }
    Ok(out)
}
