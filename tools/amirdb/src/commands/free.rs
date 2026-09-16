//! `amirdb <image> free` — report the cylinder ranges not claimed by any
//! partition, within `rdb_LoCylinder..=rdb_HiCylinder`.

use std::path::Path;

use amiga_rdb::Rdb;
use anyhow::Result;
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {}

/// The cylinder ranges no partition claims, in ascending order.
///
/// Same gap-finding rule [`RdbEditor::add_partition`]'s `Placement::Size`
/// uses internally (first fit from the lowest allocatable cylinder) —
/// this just reports the gaps instead of picking one. [`super::add::run`]'s
/// default placement takes the first of these; [`super::fill::run`]
/// takes the largest.
pub(crate) fn free_ranges(rdb: &Rdb) -> Vec<(u32, u32)> {
    let mut used: Vec<(u32, u32)> = rdb
        .partitions
        .iter()
        .filter(|p| p.high_cyl >= p.low_cyl)
        .map(|p| (p.low_cyl, p.high_cyl))
        .collect();
    used.sort_unstable();

    let mut ranges = Vec::new();
    let mut cursor = rdb.lo_cylinder;
    for (low, high) in used {
        if low > cursor {
            ranges.push((cursor, low - 1));
        }
        cursor = cursor.max(high.saturating_add(1));
    }
    if cursor <= rdb.hi_cylinder {
        ranges.push((cursor, rdb.hi_cylinder));
    }
    ranges
}

pub fn run(image: &Path, block_size: usize, _args: Args) -> Result<()> {
    let (rdb, _disk) = super::open_rdb(image, block_size)?;
    let cyl_bytes = rdb.cyl_blocks as u64 * rdb.block_bytes as u64;
    let ranges = free_ranges(&rdb);

    if ranges.is_empty() {
        println!("(no free cylinders: {}..={} is fully claimed)", rdb.lo_cylinder, rdb.hi_cylinder);
        return Ok(());
    }

    let mut total_cyls = 0u64;
    for (low, high) in &ranges {
        let cyls = *high as u64 - *low as u64 + 1;
        total_cyls += cyls;
        println!(
            "{}..={}  ({} cylinders, {} MiB)",
            low,
            high,
            cyls,
            cyls.saturating_mul(cyl_bytes) / (1024 * 1024)
        );
    }
    println!(
        "total free: {} cylinders, {} MiB (of {}..={})",
        total_cyls,
        total_cyls.saturating_mul(cyl_bytes) / (1024 * 1024),
        rdb.lo_cylinder,
        rdb.hi_cylinder
    );
    Ok(())
}
