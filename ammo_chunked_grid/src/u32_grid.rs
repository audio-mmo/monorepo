//! The `U32Grid` is the backing type for this crate, and is optimized for
//! closely spaced, small `u32` values.  The public type is `Grid`, which
//! combines this with a lookup table to produce a grid which can hold any type.
//!
//! This is a HashMap of `Chunk`s.
use std::collections::HashMap;

use crate::chunks::Chunk;
use crate::write_destination::*;

pub(crate) struct U32Grid {
    chunks: HashMap<ChunkId, Chunk>,
}

impl U32Grid {
    pub(crate) fn new() -> U32Grid {
        U32Grid {
            chunks: Default::default(),
        }
    }

    /// Read a cell of the grid.  Returns 0 for cells which don't contain data.
    pub(crate) fn read(&self, x: i64, y: i64) -> u32 {
        let dest = WriteDestination::from_coords(x, y);
        self.chunks
            .get(&dest.chunk)
            .map(|x| x.read(dest.x, dest.y))
            .unwrap_or(0)
    }

    /// Write a location in the grid, returning the old value.
    pub(crate) fn write(&mut self, x: i64, y: i64, value: u32) -> u32 {
        let dest = WriteDestination::from_coords(x, y);
        self.chunks
            .entry(dest.chunk)
            .or_insert_with(|| Chunk::new_uncompressed(0))
            .write(dest.x, dest.y, value)
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #[test]
        fn read_write(
            // The tuples are (x, y, value).
            tups in prop::collection::vec((i64::MIN..i64::MAX, i64::MIN..i64::MAX, u32::MIN..u32::MAX), 1..1000)
        ) {
            // We will compare the grid against a hashmap implementation.
            let mut good_impl: HashMap<(i64, i64), u32> = HashMap::new();
            let mut grid = U32Grid::new();

            for (x, y, val) in tups {
                let good_old = good_impl.insert((x, y), val).unwrap_or(0);
                let old = grid.write(x, y, val);
                assert_eq!(old, good_old);
            }

            // Now check that reading it all back works.
            for ((e_x, e_y), e_val) in good_impl.into_iter() {
                assert_eq!(grid.read(e_x, e_y), e_val);
            }
        }
    }
}
