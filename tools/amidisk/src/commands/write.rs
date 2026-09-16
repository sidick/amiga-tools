//! `amidisk <image> write <host-path> [ami-path] [--recursive]` — write a
//! host file (or, with `--recursive`, a directory tree) into the volume.
//!
//! Destination semantics follow `cp`: if `ami-path` names (or, once
//! resolved, turns out to be) an existing directory, the host item is
//! written inside it under its own name; otherwise `ami-path` is the
//! exact destination name, and its parent must already exist. With no
//! `ami-path` at all, the destination is the volume root plus the host
//! item's own name.
//!
//! An existing file of the same name is overwritten: shrunk to nothing,
//! then written fresh, so a file that got smaller does not keep stale
//! trailing blocks. A host directory's mtime and readonly bit carry
//! across as the Amiga entry's date and protection bits
//! (`populate::datestamp_from_system_time`, `protection_from_metadata`).

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use amiga_ffs::populate::{datestamp_from_system_time, latin1_name, protection_from_metadata};
use amiga_ffs::{EntryKind, MetaUpdate, Metadata, Mutator};
use anyhow::{bail, Context, Result};
use clap::Args as ClapArgs;

use crate::disk::FileDisk;

use super::display_name;

#[derive(ClapArgs)]
pub struct Args {
    /// File or directory on the host to write in.
    pub host_path: PathBuf,

    /// Destination inside the volume; defaults to the host path's own
    /// last component at the volume root.
    pub ami_path: Option<String>,

    /// Required to write a directory; copies it in recursively.
    #[arg(long)]
    pub recursive: bool,
}

pub fn run(image: &Path, args: Args) -> Result<()> {
    let host_path = &args.host_path;
    let md = fs::symlink_metadata(host_path)
        .with_context(|| format!("reading {}", host_path.display()))?;

    if md.is_dir() && !args.recursive {
        bail!("{}: is a directory (use --recursive)", host_path.display());
    }
    if !md.is_dir() && !md.is_file() {
        bail!("{}: not a regular file or directory", host_path.display());
    }

    let now = datestamp_from_system_time(SystemTime::now());
    let mut mutator = super::open_mutator(image)?.clock(now);

    let host_name = host_path.file_name().and_then(latin1_name).with_context(|| {
        format!("{}: name is not representable in an Amiga (Latin-1) name", host_path.display())
    })?;

    let (parent, name) = resolve_target(&mut mutator, args.ami_path.as_deref(), &host_name)?;

    if md.is_dir() {
        write_tree(&mut mutator, parent, &name, host_path)?;
    } else {
        write_file_entry(&mut mutator, parent, &name, host_path, &md)?;
    }

    super::save_back(mutator, image)
}

/// Where a host item lands: an existing directory (own name preserved),
/// an existing non-directory (overwritten under its own recorded name),
/// or a path that does not exist yet (its parent must, and the last
/// component becomes the new entry's name).
fn resolve_target(
    mutator: &mut Mutator<FileDisk>,
    ami_path: Option<&str>,
    host_name: &[u8],
) -> Result<(u64, Vec<u8>)> {
    let root = mutator.volume().root_lba();
    let Some(p) = ami_path else {
        return Ok((root, host_name.to_vec()));
    };

    let found = mutator
        .volume()
        .lookup_path(root, p.as_bytes())
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("looking up {p}"))?;

    if let Some(entry) = found {
        return Ok(if entry.kind.is_directory() {
            (entry.lba, host_name.to_vec())
        } else {
            (entry.parent as u64, entry.name.clone())
        });
    }

    let components: Vec<&str> = p.split('/').filter(|c| !c.is_empty()).collect();
    let Some((leaf, parent_comps)) = components.split_last() else {
        bail!("{p:?}: empty path");
    };

    let parent_lba = if parent_comps.is_empty() {
        root
    } else {
        let parent_path = parent_comps.join("/");
        let entry = mutator
            .volume()
            .lookup_path(root, parent_path.as_bytes())
            .map_err(|e| anyhow::anyhow!("{e}"))
            .with_context(|| format!("looking up {parent_path}"))?
            .with_context(|| format!("{parent_path}: not found"))?;
        if !entry.kind.is_directory() {
            bail!("{parent_path}: not a directory");
        }
        entry.lba
    };

    Ok((parent_lba, ami_name_bytes(leaf)?))
}

/// Create (or reuse) the directory at `parent`/`name`, then copy every
/// child of `host_dir` into it, sorted for reproducibility.
fn write_tree(mutator: &mut Mutator<FileDisk>, parent: u64, name: &[u8], host_dir: &Path) -> Result<()> {
    let existing = mutator
        .volume()
        .lookup(parent, name)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("looking up {}", display_name(name)))?;

    let dir_lba = match existing {
        Some(entry) if entry.kind.is_directory() => entry.lba,
        Some(entry) => bail!(
            "{}: exists and is not a directory ({:?})",
            display_name(name),
            entry.kind
        ),
        None => {
            let md = fs::symlink_metadata(host_dir).with_context(|| format!("reading {}", host_dir.display()))?;
            let meta = host_metadata(&md);
            mutator
                .create_dir(parent, name, &meta)
                .map_err(|e| anyhow::anyhow!("{e}"))
                .with_context(|| format!("creating directory {}", display_name(name)))?
        }
    };

    let mut children: Vec<PathBuf> = fs::read_dir(host_dir)
        .with_context(|| format!("reading {}", host_dir.display()))?
        .map(|e| e.map(|e| e.path()))
        .collect::<std::io::Result<_>>()
        .with_context(|| format!("reading {}", host_dir.display()))?;
    children.sort();

    for path in children {
        let file_name = path.file_name().and_then(latin1_name).with_context(|| {
            format!("{}: name is not representable in an Amiga (Latin-1) name", path.display())
        })?;
        // symlink_metadata, not metadata: a symlink is refused rather than
        // silently followed out of the host tree.
        let md = fs::symlink_metadata(&path).with_context(|| format!("reading {}", path.display()))?;
        if md.is_dir() {
            write_tree(mutator, dir_lba, &file_name, &path)?;
        } else if md.is_file() {
            write_file_entry(mutator, dir_lba, &file_name, &path, &md)?;
        } else {
            bail!("{}: not a regular file or directory", path.display());
        }
    }
    Ok(())
}

/// Create or overwrite one file at `parent`/`name` with `host_file`'s
/// contents, carrying its mtime and readonly bit across.
fn write_file_entry(
    mutator: &mut Mutator<FileDisk>,
    parent: u64,
    name: &[u8],
    host_file: &Path,
    md: &fs::Metadata,
) -> Result<()> {
    let data = fs::read(host_file).with_context(|| format!("reading {}", host_file.display()))?;
    let meta = host_metadata(md);

    let existing = mutator
        .volume()
        .lookup(parent, name)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("looking up {}", display_name(name)))?;

    match existing {
        None => {
            mutator
                .create_file(parent, name, &meta, &data)
                .map_err(|e| anyhow::anyhow!("{e}"))
                .with_context(|| format!("creating {}", display_name(name)))?;
        }
        Some(entry) if entry.kind == EntryKind::File => {
            // Shrink to nothing, then write the new content fresh: a
            // plain write_file at offset 0 never shortens a file (its
            // new length is the max of the old and new sizes), so a
            // truncate first is what makes this a genuine overwrite
            // rather than "grow, maybe".
            mutator
                .truncate(parent, name, 0)
                .map_err(|e| anyhow::anyhow!("{e}"))
                .with_context(|| format!("truncating {}", display_name(name)))?;
            mutator
                .write_file(parent, name, 0, &data)
                .map_err(|e| anyhow::anyhow!("{e}"))
                .with_context(|| format!("writing {}", display_name(name)))?;
            mutator
                .set_metadata(entry.lba, &MetaUpdate::new().protection(meta.protection).date(meta.date))
                .map_err(|e| anyhow::anyhow!("{e}"))
                .with_context(|| format!("updating metadata for {}", display_name(name)))?;
        }
        Some(entry) => bail!(
            "{}: exists and is not a plain file ({:?})",
            display_name(name),
            entry.kind
        ),
    }

    println!("{} ({} bytes)", display_name(name), data.len());
    Ok(())
}

fn host_metadata(md: &fs::Metadata) -> Metadata<'static> {
    Metadata::new()
        .protection(protection_from_metadata(md))
        .date(md.modified().map(datestamp_from_system_time).unwrap_or_default())
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
