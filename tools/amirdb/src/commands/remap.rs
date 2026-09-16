//! `amirdb <image> remap (--geometry <C,H,S> | --auto)` — rewrite the
//! disk's declared cylinder count in place: the "image was cloned onto
//! a differently-sized medium and the RDB still describes the old one"
//! case.
//!
//! # The API gap this command works around
//!
//! `amiga-rdb`'s only geometry-editing entry point on a populated RDB
//! is `RdbEditor::set_geometry_cylinders`, which changes `rdb_Cylinders`
//! (and `rdb_HiCylinder` with it) and *nothing else*. There is no
//! editor method that changes `rdb_Heads` or `rdb_Sectors`: doing so
//! honestly would mean recomputing every partition's `de_LowCyl`/
//! `de_HighCyl` to keep the same byte extents (a partition's cylinder
//! is its own `de_Surfaces * de_BlocksPerTrack`, not necessarily the
//! drive's), and no such recomputation exists in the crate today. So
//! this command only performs the part that *is* possible — growing or
//! shrinking the cylinder count while heads/sectors stay fixed — and
//! refuses cleanly, naming the gap, when the requested (or
//! synthesized) geometry would need to change either. That refusal is
//! the honest answer for now; a heads/sectors remap that preserves
//! partition extents would need a new `RdbEditor` method upstream.
//!
//! `--auto` re-synthesizes a geometry from the image's current size —
//! the same `synthesize_geometry` call `init` uses for a fresh image —
//! and applies it on the same terms.

use std::path::Path;

use amiga_rdb::synthesize_geometry;
use anyhow::{bail, Context, Result};
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Exact geometry `cylinders,heads,sectors`. `heads`/`sectors` must
    /// match the image's current geometry — only the cylinder count can
    /// actually change; see the module doc for why.
    #[arg(long, conflicts_with = "auto", required_unless_present = "auto")]
    pub geometry: Option<String>,

    /// Re-synthesize the geometry from the disk's current size, as
    /// `init` would for a fresh image. Applied only if the synthesized
    /// `heads`/`sectors` match the current ones; otherwise refused with
    /// the mismatch shown (see the module doc).
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

    if new_heads != heads || new_sectors != sectors {
        bail!(
            "target geometry {new_cylinders}/{new_heads}/{new_sectors} changes heads/sectors \
             from the current {old_cylinders}/{heads}/{sectors} — amiga-rdb's RdbEditor only \
             exposes set_geometry_cylinders (cylinder count only); there is no editor API to \
             remap heads or sectors on a populated RDB and keep partition extents correct (an \
             upstream gap, not a missing flag here). Pass a geometry that keeps heads/sectors \
             at {heads}/{sectors}, or use --auto only when that also synthesizes {heads}/{sectors}."
        );
    }

    editor
        .set_geometry_cylinders(new_cylinders)
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
