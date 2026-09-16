//! `amidisk <image> relabel <new-name>` — rename the volume itself.
//!
//! `amiga-ffs`'s [`Mutator`](amiga_ffs::Mutator) has no rename-the-root
//! entry point — every metadata setter it offers acts on a directory
//! entry, and the root is not one (`amiga_ffs::MutateError::IsRoot`).
//! Renaming the volume is therefore the one metadata op here that patches
//! the root block directly, the same way `boot` patches the boot area:
//! read the block, overwrite the BCPL name field, recompute the standard
//! block checksum ([`amiga_ffs::checksum_compute`]), write it back.

use std::path::Path;

use amiga_ffs::layout::{tail, CHECKSUM_INDEX, OFF_CHECKSUM, TL_ROOT_NAME};
use amiga_ffs::{checksum_compute, BlockSink, BlockSource, MAX_NAME_CLASSIC};
use anyhow::{bail, Context, Result};
use clap::Args as ClapArgs;

#[derive(ClapArgs)]
pub struct Args {
    /// The volume's new name (1..=30 bytes, no `:` or `/`).
    pub new_name: String,
}

/// The BCPL name field is 32 bytes wide (a length byte, up to 30
/// characters, one byte of padding to keep the next longword-aligned
/// field where it belongs) — zeroing all of it before writing the new,
/// possibly shorter, name means no byte of the old one survives.
const NAME_FIELD_LEN: usize = 32;

pub fn run(image: &Path, args: Args) -> Result<()> {
    let name = args.new_name.as_bytes();
    if name.is_empty() || name.len() > MAX_NAME_CLASSIC || name.contains(&b':') || name.contains(&b'/') {
        bail!("{:?} is not a valid volume name (1..={MAX_NAME_CLASSIC} bytes, no ':' or '/')", args.new_name);
    }

    let vol = super::open_volume(image)?;
    let root_lba = vol.root_lba();
    let bs = vol.block_size();
    let mut disk = vol.into_inner();

    let mut buf = vec![0u8; bs];
    disk.read_block(root_lba, &mut buf).map_err(|e| anyhow::anyhow!("{e}"))?;

    let off = tail(bs, TL_ROOT_NAME);
    buf[off..off + NAME_FIELD_LEN].fill(0);
    buf[off] = name.len() as u8;
    buf[off + 1..off + 1 + name.len()].copy_from_slice(name);

    let ck = checksum_compute(&buf, CHECKSUM_INDEX);
    buf[OFF_CHECKSUM..OFF_CHECKSUM + 4].copy_from_slice(&ck.to_be_bytes());

    disk.write_block(root_lba, &buf).map_err(|e| anyhow::anyhow!("{e}"))?;
    disk.save(image).with_context(|| format!("writing {}", image.display()))?;

    println!("{}: renamed to {:?}", image.display(), args.new_name);
    Ok(())
}
