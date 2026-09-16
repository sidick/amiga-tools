//! `amirdb <image> adjust [--rdb-blocks <n>] [--lo-cyl <n>] [--cylinders <n>]
//! [--disk-id VENDOR,PRODUCT,REV] [--controller-id VENDOR,PRODUCT,REV]` —
//! apply one or more disk-level edits in place: grow the reserved RDB
//! area, move the first cylinder available to partitions, grow the
//! declared cylinder count, or set the drive/controller identity
//! strings. Every flag maps straight onto one `RdbEditor` setter; at
//! least one is required, and each is applied and reported in the
//! order listed above regardless of the order given on the command
//! line.

use std::path::Path;

use anyhow::{bail, Context, Result};
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// New reserved-area size in blocks, counted from
    /// `rdb_RDBBlocksLo`: the area becomes
    /// `rdb_RDBBlocksLo ..= rdb_RDBBlocksLo + N - 1`. Only grows the
    /// area (`RdbEditor::expand_rdb_area`); refused if the newly
    /// claimed blocks would clip a partition already using them, or if
    /// the area would reach past the end of the disk.
    #[arg(long = "rdb-blocks")]
    pub rdb_blocks: Option<u32>,

    /// New `rdb_LoCylinder` — the first cylinder available to
    /// partitions. Refused if any partition already starts below it.
    #[arg(long = "lo-cyl")]
    pub lo_cyl: Option<u32>,

    /// New `rdb_Cylinders` (and `rdb_HiCylinder` with it) — the
    /// disk-got-bigger case, where an image was cloned onto a larger
    /// medium and the RDB still describes the old one. `heads` and
    /// `sectors` are untouched. Refused if it would shrink below a
    /// partition's last block, or past what the image actually holds.
    #[arg(long = "cylinders")]
    pub cylinders: Option<u32>,

    /// Set the drive's SCSI-INQUIRY-style identity and turn
    /// `rdb_Flags`' `DISK_ID` bit on: `VENDOR,PRODUCT,REV`.
    #[arg(long = "disk-id")]
    pub disk_id: Option<String>,

    /// Set the controller's identity, on the same terms as `--disk-id`.
    #[arg(long = "controller-id")]
    pub controller_id: Option<String>,
}

/// Split a `VENDOR,PRODUCT,REV` triple, as `--disk-id`/`--controller-id`
/// take it.
fn parse_identity(spec: &str) -> Result<(String, String, String)> {
    let parts: Vec<&str> = spec.split(',').collect();
    match parts.as_slice() {
        [v, p, r] => Ok((v.trim().to_string(), p.trim().to_string(), r.trim().to_string())),
        _ => bail!("{spec:?} is not a VENDOR,PRODUCT,REV triple"),
    }
}

pub fn run(image: &Path, block_size: usize, args: Args) -> Result<()> {
    if args.rdb_blocks.is_none()
        && args.lo_cyl.is_none()
        && args.cylinders.is_none()
        && args.disk_id.is_none()
        && args.controller_id.is_none()
    {
        bail!(
            "adjust needs at least one of --rdb-blocks, --lo-cyl, --cylinders, \
             --disk-id, --controller-id"
        );
    }

    let (mut editor, disk) = super::open_editor(image, block_size)?;

    if let Some(n) = args.rdb_blocks {
        if n == 0 {
            bail!("--rdb-blocks must be at least 1");
        }
        let lo = editor.rdb().rdb_blocks_lo;
        let old_hi = editor.rdb().rdb_blocks_hi;
        let new_hi = lo
            .checked_add(n - 1)
            .with_context(|| format!("--rdb-blocks {n}: {lo} + {n} - 1 overflows a block number"))?;
        editor
            .expand_rdb_area(new_hi)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("--rdb-blocks")?;
        println!("rdb blocks: {lo}..={old_hi} -> {lo}..={new_hi}");
    }

    if let Some(n) = args.lo_cyl {
        let old = editor.rdb().lo_cylinder;
        editor
            .set_lo_cylinder(n)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("--lo-cyl")?;
        println!("lo cylinder: {old} -> {n}");
    }

    if let Some(n) = args.cylinders {
        let old = editor.rdb().cylinders;
        editor
            .set_geometry_cylinders(n)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("--cylinders")?;
        println!("cylinders: {old} -> {n}");
    }

    if let Some(spec) = &args.disk_id {
        let (vendor, product, revision) = parse_identity(spec)?;
        editor
            .set_disk_identity(&vendor, &product, &revision)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("--disk-id")?;
        println!("disk id: {vendor} {product} rev {revision}");
    }

    if let Some(spec) = &args.controller_id {
        let (vendor, product, revision) = parse_identity(spec)?;
        editor
            .set_controller_identity(&vendor, &product, &revision)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("--controller-id")?;
        println!("controller id: {vendor} {product} rev {revision}");
    }

    let report = super::commit_editor(&editor, disk, image)?;
    println!("{}: adjusted, {} block(s) written", image.display(), report.blocks_written.len());
    Ok(())
}
