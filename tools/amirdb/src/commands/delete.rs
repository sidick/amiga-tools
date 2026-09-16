//! `amirdb <image> delete <name-or-index>` — remove a partition. Only
//! the `PART` table entry is removed; the filesystem inside the extent
//! is left exactly as it was (see [`RdbEditor::remove_partition`]).

use std::path::Path;

use anyhow::{Context, Result};
use clap::Args as ClapArgs;

use super::add::warn_if_not_clean;

#[derive(ClapArgs)]
pub struct Args {
    /// Partition index, or `pb_DriveName` (case-insensitive).
    pub selector: String,
}

pub fn run(image: &Path, block_size: usize, args: Args) -> Result<()> {
    let (mut editor, disk) = super::open_editor(image, block_size)?;
    let index = super::find_partition(editor.rdb(), &args.selector)?;
    let removed = editor.partitions()[index].clone();

    editor
        .remove_partition(index)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("removing partition {:?} from {}", args.selector, image.display()))?;

    super::commit_editor(&editor, disk, image)?;

    println!(
        "{}: removed {} (was cylinders {}..={}); its data on disk was left untouched",
        image.display(),
        removed.name,
        removed.low_cyl,
        removed.high_cyl
    );

    warn_if_not_clean(image, block_size)?;
    Ok(())
}
