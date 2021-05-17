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
    /// Create a new uncompressed chunk.  Writes start in an uncompressed chunk,
    /// then potentially become a different chunk type depending on if the
    /// caller decides to compress further.  Uncompressed chunk writes always
    /// succeed.
    pub fn new_uncompressed(default_val: u32) -> Chunk {
        Chunk::Uncompressed(Box::new(uncompressed::UncompressedChunk::filled_with(
            default_val,
        )))
    }

    pub(crate) fn read(&self, x: usize, y: usize) -> u32 {
        match self {
            Chunk::Uncompressed(c) => c.read(x, y),
        }
    }

    /// Write to the cell, returning the old value.
    pub(crate) fn write(&mut self, x: usize, y: usize, value: u32) -> u32 {
        match self {
            // The uncompressed chunk should always accept writing.
            Chunk::Uncompressed(c) => c
                .try_write(x, y, value)
                .expect("Should always be able to write to the uncompressed chunk"),
        }
    }
}
