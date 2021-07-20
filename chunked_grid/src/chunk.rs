//! A chunk, a subsection of the larger array.  Empty chunks return the default
//! value, and aren't allocated at all.

// We need these constants because Rust const generics isn't yet powerful enough
// to be abstract over the dimensions of the cells.
///
/// Should evenly divide u64::MAX.
pub(crate) const CHUNK_WIDTH: usize = 128;
pub(crate) const CHUNK_HEIGHT: usize = 128;

pub(crate) struct Chunk {
    data: [u32; CHUNK_WIDTH * CHUNK_HEIGHT],
}

impl Chunk {
    pub(crate) fn new(default_val: u32) -> Chunk {
        Chunk {
            data: [default_val; CHUNK_WIDTH * CHUNK_HEIGHT],
        }
    }

    pub(crate) fn read(&self, x: usize, y: usize) -> u32 {
        debug_assert!(x < CHUNK_WIDTH);
        debug_assert!(y < CHUNK_HEIGHT);
        self.data[y * CHUNK_WIDTH + x]
    }

    /// Write to the cell, returning the old value.
    pub(crate) fn write(&mut self, x: usize, y: usize, mut value: u32) -> u32 {
        debug_assert!(x < CHUNK_WIDTH);
        debug_assert!(y < CHUNK_HEIGHT);
        let ind = y * CHUNK_WIDTH + x;
        std::mem::swap(&mut value, &mut self.data[ind]);
        value
    }
}
