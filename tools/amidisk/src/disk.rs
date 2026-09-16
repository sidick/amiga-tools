//! The block-device seam amiga-ffs asks its consumer to provide: a whole
//! ADF/HDF image held in memory, sliced into 512-byte blocks.

use amiga_ffs::BlockSource;

/// A block index that fell outside the image.
#[derive(Debug)]
pub struct OutOfRange(pub u64);

impl std::fmt::Display for OutOfRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "block {} is outside the image", self.0)
    }
}

impl std::error::Error for OutOfRange {}

/// The whole image file, read once and sliced on demand.
pub struct FileDisk {
    data: Vec<u8>,
}

impl FileDisk {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
}

impl BlockSource for FileDisk {
    type Error = OutOfRange;

    fn block_size(&self) -> usize {
        512
    }

    fn read_block(&mut self, lba: u64, buf: &mut [u8]) -> Result<(), Self::Error> {
        let off = lba as usize * 512;
        let block = self
            .data
            .get(off..off + 512)
            .ok_or(OutOfRange(lba))?;
        buf.copy_from_slice(block);
        Ok(())
    }

    fn block_count(&self) -> Option<u64> {
        Some((self.data.len() / 512) as u64)
    }
}
