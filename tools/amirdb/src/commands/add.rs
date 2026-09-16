//! `amirdb <image> add <name> (--size <bytes> | --cyl <lo>-<hi> | --fill)
//! [...]` — add a partition to an existing RDB.

use std::path::Path;

use amiga_rdb::{Partition, PartitionSpec, Placement, Rdb};
use anyhow::{Context, Result};
use clap::Args as ClapArgs;

use super::free::free_ranges;

#[derive(ClapArgs)]
pub struct Args {
    /// `pb_DriveName` for the new partition.
    pub name: String,

    /// Size, e.g. `10Mi`, `500M`. Mutually exclusive with `--cyl`/`--fill`.
    #[arg(long, conflicts_with_all = ["cyl", "fill"])]
    pub size: Option<String>,

    /// Exact cylinder range, `lo-hi` (inclusive), e.g. `2-100`. Mutually
    /// exclusive with `--size`/`--fill`.
    #[arg(long, conflicts_with_all = ["size", "fill"])]
    pub cyl: Option<String>,

    /// Take the whole of the next free cylinder range (first fit, from
    /// the lowest allocatable cylinder) instead of a size — the same
    /// range this flag's absence defaults to.
    #[arg(long, conflicts_with_all = ["size", "cyl"])]
    pub fill: bool,

    /// `ofs`/`ffs[+intl][+dircache]`, `DOS0..DOS7`, `PDS3`-style, or
    /// `0x...`. Defaults to `DOS3` (`ffs+intl`), the library's own
    /// default `PartitionSpec` dostype.
    #[arg(long = "dos-type")]
    pub dos_type: Option<String>,

    /// Mark bootable.
    #[arg(long)]
    pub bootable: bool,

    /// `de_BootPri`; implies `--bootable`.
    #[arg(long = "boot-pri")]
    pub boot_pri: Option<i32>,

    /// Mount, but not automatically at boot.
    #[arg(long = "no-automount")]
    pub no_automount: bool,

    /// `de_MaxTransfer`.
    #[arg(long = "max-transfer")]
    pub max_transfer: Option<u32>,

    /// `de_NumBuffers`.
    #[arg(long = "num-buffers")]
    pub num_buffers: Option<u32>,

    /// `de_Reserved` — boot blocks at the start of the partition.
    #[arg(long)]
    pub reserved: Option<u32>,
}

/// Parse a `lo-hi` cylinder range, as `--cyl` takes it. Shared with
/// `addimg`, which accepts the same flag.
pub(crate) fn parse_cyl_range(spec: &str) -> Result<(u32, u32)> {
    let (lo, hi) = spec
        .split_once('-')
        .with_context(|| format!("{spec:?} is not a LO-HI cylinder range, e.g. 2-100"))?;
    Ok((
        lo.trim().parse().with_context(|| format!("{lo:?} is not a valid cylinder"))?,
        hi.trim().parse().with_context(|| format!("{hi:?} is not a valid cylinder"))?,
    ))
}

/// The mount-parameter flags `add`, `addimg` and `fill` all take, bundled
/// so [`apply_common_flags`] does not itself become the thing clippy
/// flags as too many arguments.
#[derive(Default)]
pub(crate) struct CommonFlags<'a> {
    pub dos_type: Option<&'a str>,
    pub bootable: bool,
    pub boot_pri: Option<i32>,
    pub no_automount: bool,
    pub max_transfer: Option<u32>,
    pub num_buffers: Option<u32>,
    pub reserved: Option<u32>,
}

/// Apply the mount-parameter flags every `add`/`addimg`/`fill` share on
/// top of a freshly built [`PartitionSpec`].
pub(crate) fn apply_common_flags(mut spec: PartitionSpec, flags: CommonFlags<'_>) -> Result<PartitionSpec> {
    if let Some(dos_type) = flags.dos_type {
        spec = spec.dos_type(super::parse_dostype(dos_type)?);
    }
    if flags.bootable || flags.boot_pri.is_some() {
        spec = spec.bootable(flags.boot_pri.unwrap_or(0));
    }
    spec.no_automount = flags.no_automount;
    if let Some(v) = flags.max_transfer {
        spec.max_transfer = v;
    }
    if let Some(v) = flags.num_buffers {
        spec.num_buffers = v;
    }
    if let Some(v) = flags.reserved {
        spec.reserved = v;
    }
    Ok(spec)
}

/// Re-parse `image` after a commit and warn on stderr if
/// [`Rdb::validate`] is not clean — an internal sanity check every
/// mutating command in this module group runs after saving.
pub(crate) fn warn_if_not_clean(image: &Path, block_size: usize) -> Result<()> {
    let (rdb, _disk) = super::open_rdb(image, block_size)?;
    let issues = rdb.validate();
    if !issues.is_empty() {
        eprintln!("warning: {} has {} validation issue(s) after this change:", image.display(), issues.len());
        for issue in &issues {
            eprintln!("  {issue}");
        }
    }
    Ok(())
}

/// One line describing a freshly added partition, used by `add`,
/// `addimg` and `fill` alike.
pub(crate) fn describe(rdb: &Rdb, p: &Partition) -> String {
    format!(
        "{} on cylinders {}..={} ({} MiB), dostype {}{}",
        p.name,
        p.low_cyl,
        p.high_cyl,
        p.block_len.saturating_mul(rdb.block_bytes as u64) / (1024 * 1024),
        super::dostype_str(p.dos_type),
        if p.bootable {
            format!(", bootable (boot-pri {})", p.boot_pri)
        } else {
            String::new()
        }
    )
}

pub fn run(image: &Path, block_size: usize, args: Args) -> Result<()> {
    let (mut editor, disk) = super::open_editor(image, block_size)?;

    let placement = if let Some(size) = &args.size {
        Placement::Size(super::parse_size(size)?)
    } else if let Some(cyl) = &args.cyl {
        let (low, high) = parse_cyl_range(cyl)?;
        Placement::Cylinders { low, high }
    } else {
        // Default, and `--fill`: the whole of the next free range.
        let (low, high) = free_ranges(editor.rdb())
            .into_iter()
            .next()
            .context("no free cylinders left to add a partition on")?;
        Placement::Cylinders { low, high }
    };

    let base = match placement {
        Placement::Size(bytes) => PartitionSpec::by_size(bytes),
        Placement::Cylinders { low, high } => PartitionSpec::by_cylinders(low, high),
    }
    .named(&args.name);

    let spec = apply_common_flags(
        base,
        CommonFlags {
            dos_type: args.dos_type.as_deref(),
            bootable: args.bootable,
            boot_pri: args.boot_pri,
            no_automount: args.no_automount,
            max_transfer: args.max_transfer,
            num_buffers: args.num_buffers,
            reserved: args.reserved,
        },
    )?;

    let index = editor
        .add_partition(spec)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("adding partition {:?} to {}", args.name, image.display()))?;
    let added = editor.partitions()[index].clone();

    super::commit_editor(&editor, disk, image)?;

    println!("{}: added {}", image.display(), describe(editor.rdb(), &added));

    warn_if_not_clean(image, block_size)?;
    Ok(())
}
