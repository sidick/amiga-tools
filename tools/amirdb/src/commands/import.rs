//! `amirdb <image> import <name-or-index> <host-file>` — copy a host
//! file into a partition's raw blocks verbatim, via
//! `amiga_rdb::PartitionSink`. The counterpart to `export`.
//!
//! Matches amitools' `rdbtool` semantics: this is a size-checked import,
//! not a partial one — a host file bigger than the partition is refused
//! outright rather than truncated, and a host file smaller than the
//! partition leaves the partition's own tail blocks exactly as they
//! were (never zeroed). Only the file's own final block, if its length
//! isn't a whole number of blocks, is zero-padded out to the block
//! boundary.

use std::fs;
use std::path::{Path, PathBuf};

use amiga_rdb::{BlockSink, PartitionSink};
use anyhow::{bail, Context, Result};
use clap::Args as ClapArgs;

use super::{find_partition, open_rdb};

#[derive(ClapArgs)]
pub struct Args {
    /// Partition index, or `pb_DriveName` (case-insensitive).
    pub selector: String,

    /// Host file to read.
    pub host_file: PathBuf,
}

pub fn run(image: &Path, block_size: usize, args: Args) -> Result<()> {
    let (rdb, mut disk) = open_rdb(image, block_size)?;
    let index = find_partition(&rdb, &args.selector)?;
    let partition = &rdb.partitions[index];

    let data = fs::read(&args.host_file)
        .with_context(|| format!("reading {}", args.host_file.display()))?;

    let partition_bytes = partition.block_len * block_size as u64;
    if data.len() as u64 > partition_bytes {
        bail!(
            "{} is {} bytes, which does not fit in partition {} ({} bytes across {} block(s) of {})",
            args.host_file.display(),
            data.len(),
            partition.name,
            partition_bytes,
            partition.block_len,
            block_size
        );
    }

    let block_count = data.len().div_ceil(block_size);
    let mut buf = vec![0u8; block_size];
    {
        let mut sink = PartitionSink::new(&mut disk, partition);
        for i in 0..block_count {
            let start = i * block_size;
            let end = (start + block_size).min(data.len());
            buf.fill(0);
            buf[..end - start].copy_from_slice(&data[start..end]);
            sink.write_block(i as u64, &buf)
                .map_err(|e| anyhow::anyhow!("{e}"))
                .with_context(|| format!("writing block {i} of partition {}", partition.name))?;
        }
    }

    disk.save(image)
        .with_context(|| format!("writing {}", image.display()))?;

    println!(
        "imported {} bytes ({block_count} block(s)) from {} into partition {}",
        data.len(),
        args.host_file.display(),
        partition.name
    );
    Ok(())
}
