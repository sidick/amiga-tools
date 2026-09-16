//! `amirdb <image> export <name-or-index> <host-file>` — copy a
//! partition's raw blocks out to a host file, verbatim, via
//! `amiga_rdb::PartitionSource`. The counterpart to `import`.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use amiga_rdb::{BlockSource, PartitionSource};
use anyhow::{Context, Result};
use clap::Args as ClapArgs;

use super::{find_partition, open_rdb};

#[derive(ClapArgs)]
pub struct Args {
    /// Partition index, or `pb_DriveName` (case-insensitive).
    pub selector: String,

    /// Host file to write. Overwritten if it already exists.
    pub host_file: PathBuf,
}

pub fn run(image: &Path, block_size: usize, args: Args) -> Result<()> {
    let (rdb, mut disk) = open_rdb(image, block_size)?;
    let index = find_partition(&rdb, &args.selector)?;
    let partition = &rdb.partitions[index];

    let file = File::create(&args.host_file)
        .with_context(|| format!("creating {}", args.host_file.display()))?;
    let mut out = BufWriter::new(file);

    let mut source = PartitionSource::new(&mut disk, partition);
    let mut buf = vec![0u8; block_size];
    for lba in 0..partition.block_len {
        source
            .read_block(lba, &mut buf)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .with_context(|| format!("reading block {lba} of partition {}", partition.name))?;
        out.write_all(&buf)
            .with_context(|| format!("writing {}", args.host_file.display()))?;
    }
    out.flush()
        .with_context(|| format!("writing {}", args.host_file.display()))?;

    let bytes = partition.block_len * block_size as u64;
    println!(
        "exported {bytes} bytes ({} block(s)) from partition {} to {}",
        partition.block_len,
        partition.name,
        args.host_file.display()
    );
    Ok(())
}
