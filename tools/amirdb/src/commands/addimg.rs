//! `amirdb <image> addimg <host-image> [name] [placement/flags]` — add a
//! partition sized (by default, rounded up to whole cylinders) and
//! placed as given, then copy the host file's bytes into it block by
//! block through [`PartitionSink`].

use std::fs;
use std::path::Path;
use std::path::PathBuf;

use amiga_rdb::{BlockSink, PartitionSink, PartitionSpec, Placement};
use anyhow::{bail, Context, Result};
use clap::Args as ClapArgs;

use super::add::{apply_common_flags, describe, parse_cyl_range, warn_if_not_clean, CommonFlags};
use super::free::free_ranges;

#[derive(ClapArgs)]
pub struct Args {
    /// Host file to copy into the new partition.
    pub host_image: PathBuf,

    /// `pb_DriveName`; defaults to `host_image`'s file stem.
    pub name: Option<String>,

    /// Size, e.g. `10Mi`, `500M`. Mutually exclusive with `--cyl`; must
    /// be at least the host image's own size. Defaults to the host
    /// image's size rounded up to whole cylinders.
    #[arg(long, conflicts_with = "cyl")]
    pub size: Option<String>,

    /// Exact cylinder range, `lo-hi` (inclusive).
    #[arg(long)]
    pub cyl: Option<String>,

    /// `ofs`/`ffs[+intl][+dircache]`, `DOS0..DOS7`, `PDS3`-style, or
    /// `0x...`. Defaults to `DOS3` (`ffs+intl`).
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
}

pub fn run(image: &Path, block_size: usize, args: Args) -> Result<()> {
    let data = fs::read(&args.host_image).with_context(|| format!("reading {}", args.host_image.display()))?;
    let data_len = data.len() as u64;

    let name = match &args.name {
        Some(name) => name.clone(),
        None => args
            .host_image
            .file_stem()
            .and_then(|s| s.to_str())
            .map(String::from)
            .with_context(|| {
                format!("cannot derive a partition name from {:?}; pass one explicitly", args.host_image)
            })?,
    };

    let (mut editor, mut disk) = super::open_editor(image, block_size)?;

    let placement = if let Some(cyl) = &args.cyl {
        let (low, high) = parse_cyl_range(cyl)?;
        Placement::Cylinders { low, high }
    } else if let Some(size) = &args.size {
        let bytes = super::parse_size(size)?;
        if bytes < data_len {
            bail!("--size {bytes} is smaller than {} ({data_len} bytes)", args.host_image.display());
        }
        Placement::Size(bytes)
    } else {
        // Default: exactly enough whole cylinders to hold the file,
        // rounded up — unlike `Placement::Size`, which floors, a copy
        // must never end up short of room for the last block.
        let rdb = editor.rdb();
        let cyl_bytes = rdb.cyl_blocks as u64 * rdb.block_bytes as u64;
        if cyl_bytes == 0 {
            bail!("{}: disk geometry describes a zero-block cylinder", image.display());
        }
        let need_cyls = data_len.div_ceil(cyl_bytes).max(1);
        let (low, _) = free_ranges(rdb)
            .into_iter()
            .find(|(low, high)| (*high as u64 - *low as u64 + 1) >= need_cyls)
            .with_context(|| format!("no free run of {need_cyls} cylinders for a {data_len}-byte image"))?;
        let high = (low as u64 + need_cyls - 1) as u32;
        Placement::Cylinders { low, high }
    };

    let base = match placement {
        Placement::Size(bytes) => PartitionSpec::by_size(bytes),
        Placement::Cylinders { low, high } => PartitionSpec::by_cylinders(low, high),
    }
    .named(&name);

    let spec = apply_common_flags(
        base,
        CommonFlags {
            dos_type: args.dos_type.as_deref(),
            bootable: args.bootable,
            boot_pri: args.boot_pri,
            no_automount: args.no_automount,
            ..Default::default()
        },
    )?;

    let index = editor
        .add_partition(spec)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("adding partition {name:?} to {}", image.display()))?;
    let added = editor.partitions()[index].clone();

    if added.block_len.saturating_mul(disk.block_size_raw() as u64) < data_len {
        bail!(
            "{}: the partition ended up too small to hold {} ({data_len} bytes)",
            image.display(),
            args.host_image.display()
        );
    }

    {
        let block_size = disk.block_size_raw();
        let mut sink = PartitionSink::new(&mut disk, &added);
        let mut lba = 0u64;
        let mut offset = 0usize;
        while offset < data.len() {
            let end = (offset + block_size).min(data.len());
            let mut buf = vec![0u8; block_size];
            buf[..end - offset].copy_from_slice(&data[offset..end]);
            sink.write_block(lba, &buf)
                .map_err(|e| anyhow::anyhow!("{e}"))
                .with_context(|| format!("writing block {lba} of {name} on {}", image.display()))?;
            lba += 1;
            offset = end;
        }
    }

    super::commit_editor(&editor, disk, image)?;

    println!(
        "{}: added {}, copied {} ({data_len} bytes)",
        image.display(),
        describe(editor.rdb(), &added),
        args.host_image.display()
    );

    warn_if_not_clean(image, block_size)?;
    Ok(())
}
