//! `amirdb <image> fsadd <host-binary> --dos-type <spec> [--version X.Y]
//! [--stack N] [--priority N] [--replace]` — add (or replace) a loadable
//! filesystem driver.

use std::fs;
use std::path::{Path, PathBuf};

use amiga_rdb::FileSystemSpec;
use anyhow::{bail, Context, Result};
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// Host file holding the driver binary (hunk format), to embed.
    pub host_binary: PathBuf,

    /// `fhb_DosType` this driver serves — `ofs`/`ffs[+intl][+dircache]`,
    /// `DOS0..DOS7`, `PDS3`-style, or `0x...`.
    #[arg(long = "dos-type")]
    pub dos_type: String,

    /// `fhb_Version` as `major.minor`, e.g. `1.2`.
    #[arg(long)]
    pub version: Option<String>,

    /// `fhb_StackSize`, patched into the handler's device node.
    #[arg(long)]
    pub stack: Option<u32>,

    /// `fhb_Priority`, patched into the handler's device node.
    #[arg(long)]
    pub priority: Option<i32>,

    /// If a filesystem for this dostype already exists, replace it
    /// instead of refusing.
    #[arg(long)]
    pub replace: bool,
}

/// Parse `major.minor`.
fn parse_version(spec: &str) -> Result<(u16, u16)> {
    let (major, minor) = spec
        .split_once('.')
        .with_context(|| format!("{spec:?} is not a valid version, e.g. 1.2"))?;
    Ok((
        major.parse().with_context(|| format!("{major:?} is not a valid major version"))?,
        minor.parse().with_context(|| format!("{minor:?} is not a valid minor version"))?,
    ))
}

pub fn run(image: &Path, block_size: usize, args: Args) -> Result<()> {
    let dos_type = super::parse_dostype(&args.dos_type)?;
    let binary = fs::read(&args.host_binary)
        .with_context(|| format!("reading {}", args.host_binary.display()))?;

    let mut spec = FileSystemSpec::new(dos_type, binary);
    if let Some(v) = &args.version {
        let (major, minor) = parse_version(v)?;
        spec = spec.version(major, minor);
    }
    if let Some(stack) = args.stack {
        spec = spec.stack_size(stack);
    }
    if let Some(priority) = args.priority {
        spec = spec.priority(priority);
    }

    let (mut editor, disk) = super::open_editor(image, block_size)?;
    let existing = editor.rdb().filesystems.iter().position(|f| f.dos_type == dos_type);

    let index = match existing {
        Some(index) if !args.replace => {
            bail!(
                "a filesystem for dostype {} already exists at index {index}; pass --replace to overwrite it",
                super::dostype_str(dos_type)
            );
        }
        Some(index) => {
            editor
                .replace_filesystem(index, spec)
                .map_err(|e| anyhow::anyhow!("{e}"))
                .context("replacing filesystem")?;
            index
        }
        None => editor
            .add_filesystem(spec)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("adding filesystem")?,
    };

    super::commit_editor(&editor, disk, image)?;

    // Re-open to report the header as committed — block addresses are
    // only assigned during commit, so the pre-commit `FileSysHeader` in
    // `editor.rdb()` still carries placeholder LBAs.
    let (rdb, _disk) = super::open_rdb(image, block_size)?;
    let f = &rdb.filesystems[index];
    println!(
        "FSHD at block {}  {}  version {}.{}  {}",
        f.fshd_block,
        super::dostype_str(f.dos_type),
        f.version_major(),
        f.version_minor(),
        if f.seg_list_blocks == amiga_rdb::CHAIN_END {
            String::from("no LSEG chain")
        } else {
            format!("LSEG chain head block {}", f.seg_list_blocks)
        },
    );

    Ok(())
}
