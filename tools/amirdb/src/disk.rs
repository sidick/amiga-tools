//! The block-device seam amiga-rdb asks its consumer to provide: a whole
//! disk image held in memory, sliced into fixed-size blocks, readable
//! and writable in place. Mirrors amidisk's `FileDisk`, adapted to
//! amiga-rdb's `BlockSource`/`BlockSink` traits (which happen to share
//! `read_block`/`write_block`'s signature with amiga-ffs's, so the two
//! impls below read almost the same).

use std::fs;
use std::path::Path;

use amiga_rdb::{BlockSink, BlockSource};

/// A block index that fell outside the image.
#[derive(Debug)]
pub struct DiskError(pub u64);

impl std::fmt::Display for DiskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "block {} is outside the image", self.0)
    }
}

impl std::error::Error for DiskError {}

/// The default block size for a freshly created image: what every RDB
/// image amitools' rdbtool produces uses.
pub const DEFAULT_BLOCK_SIZE: usize = 512;

/// The whole image file, held in memory and sliced into `block_size`
/// chunks. Implements both [`BlockSource`] and [`BlockSink`], so a
/// single `FileDisk` can be opened, mutated (via `RdbEditor`) and saved
/// back.
pub struct FileDisk {
    data: Vec<u8>,
    block_size: usize,
}

impl FileDisk {
    /// Wrap already-loaded bytes at the given block size.
    pub fn new(data: Vec<u8>, block_size: usize) -> Self {
        Self { data, block_size }
    }

    /// Read a whole image file into memory at `block_size`. Unlike an
    /// ADF/HDF image, an RDB carries its own `rdb_BlockBytes`, but that
    /// is only knowable *after* parsing — so the caller (typically the
    /// CLI's `--block-size` flag) still supplies the size to load at.
    pub fn load(path: &Path, block_size: usize) -> std::io::Result<Self> {
        let data = fs::read(path)?;
        Ok(Self::new(data, block_size))
    }

    /// A blank, zero-filled image of `block_count` blocks.
    pub fn new_zeroed(block_count: u64, block_size: usize) -> Self {
        let len = block_count as usize * block_size;
        Self::new(vec![0u8; len], block_size)
    }

    /// The image's current block size. Not yet called anywhere in this
    /// crate — `init` reads it off the sink via `BlockSink::block_size`
    /// instead — but useful for a phase-2 command that needs it without
    /// going through the trait.
    #[allow(dead_code)]
    pub fn block_size_raw(&self) -> usize {
        self.block_size
    }

    /// The image's total size in bytes.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Write the image out atomically: to a temp file beside `path`,
    /// then renamed over it, so a crash or a failed write never leaves
    /// `path` half-written.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let dir = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
        let file_name = path.file_name().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "path has no file name")
        })?;
        let mut tmp_name = file_name.to_os_string();
        tmp_name.push(".amirdb-tmp");
        let tmp_path = dir.join(tmp_name);

        fs::write(&tmp_path, &self.data)?;
        fs::rename(&tmp_path, path)?;
        Ok(())
    }
}

impl BlockSource for FileDisk {
    type Error = DiskError;

    fn block_size(&self) -> usize {
        self.block_size
    }

    fn read_block(&mut self, lba: u64, buf: &mut [u8]) -> Result<(), Self::Error> {
        let bs = self.block_size;
        let off = lba as usize * bs;
        let block = self.data.get(off..off + bs).ok_or(DiskError(lba))?;
        buf.copy_from_slice(block);
        Ok(())
    }

    fn block_count(&self) -> Option<u64> {
        Some((self.data.len() / self.block_size) as u64)
    }
}

impl BlockSink for FileDisk {
    type Error = DiskError;

    fn block_size(&self) -> usize {
        self.block_size
    }

    fn write_block(&mut self, lba: u64, buf: &[u8]) -> Result<(), Self::Error> {
        let bs = self.block_size;
        let off = lba as usize * bs;
        let block = self.data.get_mut(off..off + bs).ok_or(DiskError(lba))?;
        block.copy_from_slice(buf);
        Ok(())
    }

    fn block_count(&self) -> Option<u64> {
        Some((self.data.len() / self.block_size) as u64)
    }
}
