//! `amirdb <image> fsdelete <selector>` — remove a loadable filesystem
//! driver. `de_DosType` on any partition that matched it is left alone
//! (see `RdbEditor::remove_filesystem`'s docs on why); this command
//! only warns, by name, about the partitions now left without a driver
//! the ROM may not know either.

use std::path::Path;

use anyhow::{Context, Result};
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Filesystem index, or a dostype spec (see `add --dos-type`).
    pub selector: String,
}

pub fn run(image: &Path, block_size: usize, args: Args) -> Result<()> {
    let (mut editor, disk) = super::open_editor(image, block_size)?;
    let index = super::find_filesystem(editor.rdb(), &args.selector)?;
    let dos_type = editor.rdb().filesystems[index].dos_type;

    let affected = editor
        .remove_filesystem(index)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .context("removing filesystem")?;

    if !affected.is_empty() {
        let names: Vec<String> = affected
            .iter()
            .map(|&i| editor.rdb().partitions[i].name.clone())
            .collect();
        eprintln!(
            "warning: {} still declares dostype {}; removed its driver anyway ({})",
            if names.len() == 1 { "partition" } else { "partitions" },
            super::dostype_str(dos_type),
            names.join(", "),
        );
    }

    super::commit_editor(&editor, disk, image)?;

    Ok(())
}
