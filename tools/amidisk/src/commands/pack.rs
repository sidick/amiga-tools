//! `amidisk <image> pack <src-dir> [--size <spec>] [--dos-type <spec>]
//! [--name <vol>] [--force]` — build a fresh image from a host directory
//! tree via `amiga_ffs::Populator`.
//!
//! `<src-dir>` is normally the directory `unpack` produced, and this is
//! the other half of that round trip: if `<src-dir>.amimeta` exists (see
//! `unpack`'s module documentation for the format), its metadata — every
//! entry's protection, date and comment, plus the volume's own name and
//! dostype unless a flag overrides them — is applied exactly, and
//! entries are inserted in the *reverse* of the order the sidecar
//! recorded them, which undoes `Populator`'s head-insertion and
//! reproduces the original hash-chain (and so `list`) order. A host file
//! or directory with no sidecar entry (no sidecar at all, or a tree
//! that has grown since `unpack`) gets protection/dates derived from the
//! host file the way `amiga_ffs::populate::add_host_tree` derives them,
//! and no comment.
//!
//! `<src-dir>.bootblock`, if present, is restored onto the finished
//! image with its checksum fixed up (see `unpack::write_bootblock`).
//!
//! With no `--size`, the smallest standard floppy the tree fits in is
//! used, falling back to a doubling sequence of HDF sizes; `Populator`
//! itself is the judge of "fits", via trial population.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use amiga_ffs::{FormatError, FormatOptions, Metadata, Populator, PopulateError, Variant};
use anyhow::{anyhow, bail, Context, Result};
use clap::Args as ClapArgs;

use super::create::{ADF_SIZE, HD_SIZE};
use super::display_name;
use super::unpack::{self, MetaEntry};
use crate::disk::{FileDisk, DEFAULT_BLOCK_SIZE};

#[derive(ClapArgs)]
pub struct Args {
    /// Host directory tree to copy in (normally what `unpack` produced).
    pub src_dir: PathBuf,

    /// Image size, e.g. `880K`, `1760K`, `4M`. Without this, the
    /// smallest size (standard floppy, then a doubling HDF sequence)
    /// the tree actually fits in is chosen automatically.
    #[arg(long)]
    pub size: Option<String>,

    /// `ofs`, `ffs`, `ffs+intl`, `ffs+intl+dircache`, `DOS0`..`DOS7`, or
    /// a raw dostype. Overrides the sidecar's own, if any; defaults to
    /// `ffs` with no sidecar.
    #[arg(long = "dos-type")]
    pub dos_type: Option<String>,

    /// The volume name. Overrides the sidecar's own, if any; defaults
    /// to `src_dir`'s own name with no sidecar.
    #[arg(long)]
    pub name: Option<String>,

    /// Overwrite `image` if it already exists.
    #[arg(long)]
    pub force: bool,
}

pub fn run(image: &Path, args: Args) -> Result<()> {
    if image.exists() && !args.force {
        bail!("{} already exists; pass --force to overwrite it", image.display());
    }
    if !args.src_dir.is_dir() {
        bail!("{} is not a directory", args.src_dir.display());
    }

    let sidecar_path = args.src_dir.with_extension("amimeta");
    let sidecar = sidecar_path
        .is_file()
        .then(|| unpack::read_sidecar(&sidecar_path))
        .transpose()?;

    let name: Vec<u8> = match &args.name {
        Some(n) => latin1_bytes(n)?,
        None => match &sidecar {
            Some(s) => s.volume_name.clone(),
            None => {
                let base = args
                    .src_dir
                    .file_name()
                    .with_context(|| format!("{}: cannot derive a volume name; pass --name", args.src_dir.display()))?;
                amiga_ffs::populate::latin1_name(base)
                    .with_context(|| format!("{base:?} is not representable in Latin-1; pass --name"))?
            }
        },
    };
    let variant = match (&args.dos_type, &sidecar) {
        (Some(spec), _) => super::parse_variant(spec)?,
        (None, Some(s)) => Variant::from_dostype(s.dostype)
            .with_context(|| format!("sidecar dostype {:#010x} is not DOS\\0..DOS\\7", s.dostype))?,
        (None, None) => Variant::Ffs,
    };
    let created = sidecar.as_ref().map_or_else(Default::default, |s| s.created);

    // Group the sidecar's entries by parent path, keeping each group in
    // the order the sidecar recorded them (`read_dir`'s own order) --
    // reversed at insertion time in `populate_tree`, per the module
    // documentation above.
    let mut by_parent: HashMap<Vec<u8>, Vec<&MetaEntry>> = HashMap::new();
    if let Some(s) = &sidecar {
        for e in &s.entries {
            by_parent.entry(unpack::parent_of(&e.path).to_vec()).or_default().push(e);
        }
    }

    let boot_path = args.src_dir.with_extension("bootblock");
    let boot_area = boot_path.is_file().then(|| fs::read(&boot_path)).transpose()?;

    let sizes: Vec<u64> = match &args.size {
        Some(s) => {
            let bytes = super::parse_size(s)?;
            if bytes % DEFAULT_BLOCK_SIZE as u64 != 0 {
                bail!("{bytes} bytes is not a whole number of {DEFAULT_BLOCK_SIZE}-byte blocks");
            }
            vec![bytes]
        }
        None => candidate_sizes().collect(),
    };
    let explicit_size = args.size.is_some();

    let mut built: Option<FileDisk> = None;
    for size in sizes {
        if size % DEFAULT_BLOCK_SIZE as u64 != 0 {
            continue;
        }
        let block_count = size / DEFAULT_BLOCK_SIZE as u64;
        let disk = FileDisk::new_zeroed(block_count, DEFAULT_BLOCK_SIZE);
        let opts = FormatOptions::new(variant, block_count, &name).created(created);

        let mut pop = match Populator::new(disk, &opts) {
            Ok(pop) => pop,
            Err(e) if !explicit_size && too_small(&e) => continue,
            Err(e) => bail!("formatting a {size}-byte image: {e}"),
        };
        let root = pop.root_lba();
        match populate_tree(&mut pop, root, &args.src_dir, &[], &by_parent) {
            Ok(()) => {
                built = Some(pop.finish().map_err(|e| anyhow!("{e}")).with_context(|| "finishing the volume".to_string())?);
                break;
            }
            Err(e) if !explicit_size && too_small_anyhow(&e) => continue,
            Err(e) => return Err(e),
        }
    }
    let mut disk = built.with_context(|| "no candidate image size fit the source tree; pass --size explicitly".to_string())?;

    if let Some(area) = &boot_area {
        unpack::write_bootblock(&mut disk, area, variant)?;
    }

    disk.save(image).with_context(|| format!("writing {}", image.display()))?;
    println!("{} <- {} ({:?}, {})", image.display(), args.src_dir.display(), variant, display_name(&name));
    Ok(())
}

/// Standard floppy sizes, then a doubling HDF sequence up to 4 TB
/// (`amiga_ffs`'s own 32-bit block-pointer ceiling is far larger than
/// any host is going to hand this tool a directory tree for).
fn candidate_sizes() -> impl Iterator<Item = u64> {
    let mut mb = 4u64 * 1024 * 1024;
    let growing = std::iter::from_fn(move || {
        if mb > 4 * 1024 * 1024 * 1024 * 1024 {
            None
        } else {
            let v = mb;
            mb *= 2;
            Some(v)
        }
    });
    [ADF_SIZE, HD_SIZE].into_iter().chain(growing)
}

/// Whether a `Populator::new` (i.e. `format`) failure means "this
/// candidate size is too small", as opposed to a real problem worth
/// stopping for.
fn too_small<E>(e: &PopulateError<E>) -> bool {
    matches!(e, PopulateError::VolumeFull { .. })
        || matches!(e, PopulateError::Format(FormatError::VolumeTooSmall { .. }))
        || matches!(e, PopulateError::LbaOutOfRange { .. })
}

/// Same question, once the error has already been flattened into
/// `anyhow::Error` by [`populate_tree`]'s `?` — matched on the rendered
/// message, since the concrete `PopulateError` no longer survives the
/// conversion. A false negative here just means falling through to
/// "no size fit" with the real error preserved, which is always safe.
fn too_small_anyhow(e: &anyhow::Error) -> bool {
    e.chain().any(|c| {
        let msg = c.to_string();
        msg.contains("no free block left") || msg.contains("cannot hold an empty")
    })
}

/// Populate one directory, in the sidecar's original order reversed
/// (see the module documentation), falling back to a sorted host
/// listing for anything the sidecar does not mention.
fn populate_tree(
    pop: &mut Populator<FileDisk>,
    dst_dir: u64,
    host_dir: &Path,
    ami_prefix: &[u8],
    by_parent: &HashMap<Vec<u8>, Vec<&MetaEntry>>,
) -> Result<()> {
    let names: Vec<Vec<u8>> = if let Some(children) = by_parent.get(ami_prefix) {
        children.iter().rev().map(|m| unpack::last_component(&m.path).to_vec()).collect()
    } else {
        let mut names = Vec::new();
        for entry in fs::read_dir(host_dir).with_context(|| format!("reading {}", host_dir.display()))? {
            let entry = entry?;
            let name = amiga_ffs::populate::latin1_name(&entry.file_name())
                .with_context(|| format!("{}: not representable in Latin-1", entry.path().display()))?;
            names.push(name);
        }
        names.sort();
        names
    };

    for name in names {
        let host_path = host_dir.join(display_name(&name));
        let ami_path = join_path(ami_prefix, &name);
        let ami_display = display_name(&ami_path);

        let sidecar_entry = by_parent.get(ami_prefix).and_then(|v| v.iter().find(|m| m.path == ami_path));
        let md = fs::symlink_metadata(&host_path).with_context(|| format!("reading {}", host_path.display()))?;
        let (protection, date, comment) = match sidecar_entry {
            Some(m) => (m.protection, m.date, m.comment.clone()),
            None => (
                amiga_ffs::populate::protection_from_metadata(&md),
                md.modified().map(amiga_ffs::populate::datestamp_from_system_time).unwrap_or_default(),
                Vec::new(),
            ),
        };
        let meta = Metadata::new().protection(protection).date(date).comment(&comment);

        if md.is_dir() {
            let child = pop
                .create_dir(dst_dir, &name, &meta)
                .map_err(|e| anyhow!("{e}"))
                .with_context(|| format!("creating directory {ami_display}"))?;
            populate_tree(pop, child, &host_path, &ami_path, by_parent)?;
        } else if md.is_file() {
            let mut file = fs::File::open(&host_path).with_context(|| format!("opening {}", host_path.display()))?;
            let mut failure: Option<std::io::Error> = None;
            pop.create_file_with(dst_dir, &name, &meta, |buf| {
                use std::io::Read;
                match file.read(buf) {
                    Ok(n) => n,
                    Err(e) => {
                        failure.get_or_insert(e);
                        0
                    }
                }
            })
            .map_err(|e| anyhow!("{e}"))
            .with_context(|| format!("writing {ami_display}"))?;
            if let Some(e) = failure {
                return Err(e).with_context(|| format!("reading {}", host_path.display()));
            }
        } else {
            bail!("{}: not a regular file or directory", host_path.display());
        }
    }
    Ok(())
}

fn join_path(prefix: &[u8], name: &[u8]) -> Vec<u8> {
    let mut path = Vec::with_capacity(prefix.len() + 1 + name.len());
    path.extend_from_slice(prefix);
    if !path.is_empty() {
        path.push(b'/');
    }
    path.extend_from_slice(name);
    path
}

/// A CLI-supplied name, e.g. `--name`, as Latin-1 bytes.
fn latin1_bytes(s: &str) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len());
    for ch in s.chars() {
        let c = ch as u32;
        if c > 0xFF {
            bail!("{s:?} is not representable in Latin-1 (an Amiga name's own encoding)");
        }
        out.push(c as u8);
    }
    Ok(out)
}
