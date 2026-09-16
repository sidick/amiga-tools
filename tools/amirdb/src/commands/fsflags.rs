//! `amirdb <image> fsflags <selector> [--set FLAG,...] [--clear FLAG,...]
//! [--list]` — inspect or rewrite which device-node fields a loadable
//! filesystem driver's `FSHD` patches (`fhb_PatchFlags`, `amiga_rdb::fshd_patch`).
//!
//! `--set`/`--clear` only ever flip bits in `fhb_PatchFlags` itself —
//! they do not change the underlying field values (`fhb_StackSize` and
//! friends), which is the same distinction `FileSysHeader`'s `Option`
//! fields draw: `None` (bit clear) is not the same as "override with
//! zero". Setting a flag whose field was never given a value (by
//! `fsadd --stack`/`--priority`, or by the driver this FSHD started
//! life with) patches a zero into that device-node field — the same
//! thing amitools' rdbtool does when asked for a bare flag with no
//! value.
//!
//! Uses [`amiga_rdb::RdbEditor::set_patch_flags`], which patches the
//! `fhb_PatchFlags` longword in the `FSHD` block directly. The driver
//! binary is never read back or rewritten, and the `LSEG` chain is
//! untouched — before this method existed, flipping one bit meant
//! reconstructing a whole `FileSystemSpec` (driver binary included) and
//! rewriting the entire chain through `replace_filesystem`.

use std::path::Path;

use amiga_rdb::fshd_patch;
use anyhow::{bail, Context, Result};
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Filesystem index, or a dostype spec (see `add --dos-type`).
    pub selector: String,

    /// Comma-separated flag names to turn on.
    #[arg(long, value_delimiter = ',')]
    pub set: Vec<String>,

    /// Comma-separated flag names to turn off.
    #[arg(long, value_delimiter = ',')]
    pub clear: Vec<String>,

    /// Show the current flags. The default when neither `--set` nor
    /// `--clear` is given.
    #[arg(long)]
    pub list: bool,
}

/// The `fshd_patch` bits, lowercase names derived from their NDK field
/// names (`fhb_StackSize` -> `stack_size`, ...).
const FLAG_NAMES: &[(&str, u32)] = &[
    ("type", fshd_patch::TYPE),
    ("task", fshd_patch::TASK),
    ("lock", fshd_patch::LOCK),
    ("handler", fshd_patch::HANDLER),
    ("stack_size", fshd_patch::STACK_SIZE),
    ("priority", fshd_patch::PRIORITY),
    ("startup", fshd_patch::STARTUP),
    ("seg_list", fshd_patch::SEG_LIST),
    ("global_vec", fshd_patch::GLOBAL_VEC),
];

fn flag_bit(name: &str) -> Result<u32> {
    FLAG_NAMES
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, bit)| *bit)
        .with_context(|| {
            let names: Vec<&str> = FLAG_NAMES.iter().map(|(n, _)| *n).collect();
            format!("unknown flag {name:?} (known: {})", names.join(", "))
        })
}

fn print_flags(patch_flags: u32) {
    let set: Vec<&str> = FLAG_NAMES
        .iter()
        .filter(|(_, bit)| patch_flags & bit != 0)
        .map(|(name, _)| *name)
        .collect();
    let unknown = patch_flags & !FLAG_NAMES.iter().fold(0, |acc, (_, bit)| acc | bit);
    if set.is_empty() && unknown == 0 {
        println!("(no flags set)");
    } else if set.is_empty() {
        println!("(no known flags set; unmodelled bits 0x{unknown:x})");
    } else if unknown == 0 {
        println!("{}", set.join(","));
    } else {
        println!("{}  (unmodelled bits 0x{unknown:x})", set.join(","));
    }
}

pub fn run(image: &Path, block_size: usize, args: Args) -> Result<()> {
    if args.set.is_empty() && args.clear.is_empty() {
        let (rdb, _disk) = super::open_rdb(image, block_size)?;
        let index = super::find_filesystem(&rdb, &args.selector)?;
        print_flags(rdb.filesystems[index].patch_flags);
        return Ok(());
    }

    let mut set_mask = 0u32;
    for name in &args.set {
        set_mask |= flag_bit(name)?;
    }
    let mut clear_mask = 0u32;
    for name in &args.clear {
        clear_mask |= flag_bit(name)?;
    }
    if set_mask & clear_mask != 0 {
        let both: Vec<&str> = FLAG_NAMES
            .iter()
            .filter(|(_, bit)| set_mask & clear_mask & bit != 0)
            .map(|(name, _)| *name)
            .collect();
        bail!("flag(s) {} given to both --set and --clear", both.join(","));
    }

    let (mut editor, disk) = super::open_editor(image, block_size)?;
    let index = super::find_filesystem(editor.rdb(), &args.selector)?;
    let f = editor.rdb().filesystems[index].clone();

    let new_flags = (f.patch_flags | set_mask) & !clear_mask;
    editor
        .set_patch_flags(index, new_flags)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .context("rewriting filesystem flags")?;

    super::commit_editor(&editor, disk, image)?;

    let (rdb, _disk) = super::open_rdb(image, block_size)?;
    print_flags(rdb.filesystems[index].patch_flags);

    Ok(())
}
