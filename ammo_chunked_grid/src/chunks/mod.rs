mod uncompressed;

// We need these constants because Rust const generics isn't yet powerful enough
// to be abstract over the dimensions of the cells.
///
/// Should evenly divide u64::MAX.
pub(crate) const CHUNK_WIDTH: usize = 128;
pub(crate) const CHUNK_HEIGHT: usize = 128;

pub(crate) enum Chunk {
    Uncompressed(Box<uncompressed::UncompressedChunk>),
}

impl Chunk {
    pub(crate) fn read(&self, x: usize, y: usize) -> u32 {
        match self {
            Chunk::Uncompressed(c) => c.read(x, y),
        }
    }

    /// Write to the cell, returning the old value.
    pub(crate) fn try_write(&mut self, x: usize, y: usize, value: u32) -> u32 {
        match self {
            // The uncompressed chunk should always accept writing.
            Chunk::Uncompressed(c) => c
                .try_write(x, y, value)
                .expect("Should always be able to write to the uncompressed chunk"),
        }
    }
}
