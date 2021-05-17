//! An uncompressed cell holds u32 data as-is, with no compression.

use super::*;

pub(crate) struct UncompressedChunk {
    data: [u32; CHUNK_WIDTH * CHUNK_HEIGHT],
}

impl UncompressedChunk {
    pub(crate) fn filled_with(value: u32) -> UncompressedChunk {
        UncompressedChunk {
            data: [value; CHUNK_WIDTH * CHUNK_HEIGHT],
        }
    }

    pub(crate) fn read(&self, x: usize, y: usize) -> u32 {
        debug_assert!(x < CHUNK_WIDTH);
        debug_assert!(y < CHUNK_HEIGHT);
        let ind = y * CHUNK_WIDTH + x;
        self.data[ind]
    }

    pub(crate) fn try_write(&mut self, x: usize, y: usize, value: u32) -> Option<u32> {
        debug_assert!(x < CHUNK_WIDTH);
        debug_assert!(y < CHUNK_HEIGHT);
        let ind = y * CHUNK_WIDTH + x;
        let old = self.data[ind];
        self.data[ind] = value;
        Some(old)
    }
}
