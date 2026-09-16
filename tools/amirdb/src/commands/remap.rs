//! `amirdb <image> remap (--geometry <C,H,S> | --auto)` — rewrite the
//! disk's declared geometry in place: the "image was cloned onto a
//! differently-sized (or differently-shaped) medium and the RDB still
//! describes the old one" case.
//!
//! Uses [`amiga_rdb::RdbEditor::remap_geometry`] for every case, cylinder
//! count alone included — the method itself delegates to
//! `set_geometry_cylinders` when heads/sectors are unchanged, so this
//! command does not need to special-case that itself. Changing
//! heads/sectors moves what a partition's `de_LowCyl`/`de_HighCyl` mean,
//! so `remap_geometry` recomputes every partition's extent to keep its
//! actual byte range — `start_lba..start_lba + block_len` — identical;
//! when that is impossible (a partition's start or length does not
//! divide evenly by the new cylinder size), it refuses with
//! [`amiga_rdb::EditError::RemapMisaligned`] rather than move or lose a
//! byte, and this command surfaces that error as-is.
//!
//! `--auto` re-synthesizes a geometry from the image's current size —
//! the same `synthesize_geometry` call `init` uses for a fresh image —
//! and applies it on the same terms.

use std::path::Path;

use amiga_rdb::{synthesize_geometry, Geometry};
use anyhow::{Context, Result};
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Exact geometry `cylinders,heads,sectors`.
    #[arg(long, conflicts_with = "auto", required_unless_present = "auto")]
    pub geometry: Option<String>,

    /// Re-synthesize the geometry from the disk's current size, as
    /// `init` would for a fresh image, then remap to it.
    #[arg(long, conflicts_with = "geometry", required_unless_present = "geometry")]
    pub auto: bool,
}

pub fn run(image: &Path, block_size: usize, args: Args) -> Result<()> {
    let (mut editor, disk) = super::open_editor(image, block_size)?;

    let (old_cylinders, heads, sectors) = {
        let rdb = editor.rdb();
        (rdb.cylinders, rdb.heads, rdb.sectors)
    };

    let (new_cylinders, new_heads, new_sectors) = if let Some(spec) = &args.geometry {
        super::parse_geometry(spec)?
    } else {
        let total_bytes = disk.len() as u64;
        let g = synthesize_geometry(total_bytes, block_size)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .with_context(|| format!("synthesizing a geometry for {total_bytes} bytes"))?;
        (g.cylinders, g.heads, g.sectors)
    };

    editor
        .remap_geometry(Geometry {
            cylinders: new_cylinders,
            heads: new_heads,
            sectors: new_sectors,
            block_size,
        })
        .map_err(|e| anyhow::anyhow!("{e}"))
        .context("remap")?;

    let report = super::commit_editor(&editor, disk, image)?;
    println!(
        "{}: geometry {old_cylinders}/{heads}/{sectors} -> {new_cylinders}/{new_heads}/{new_sectors}, \
         {} block(s) written",
        image.display(),
        report.blocks_written.len()
    );
    Ok(())
}
