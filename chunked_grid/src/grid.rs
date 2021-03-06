//! A grid of `u32`.
//!
//! This is a HashMap of `Chunk`s.
use ammo_cached_hash_map::CachedHashMap;
use ammo_nzslab::{Slab, SlabHandle};

use crate::chunk::Chunk;
use crate::write_destination::*;

// Don't derive debug because nothing good can ever come from printing gigabytes of text in the common case.
#[derive(Default)]
pub struct Grid {
    chunks: CachedHashMap<ChunkId, SlabHandle<Chunk>>,
    chunk_slab: Slab<Chunk>,
}

impl Grid {
    pub fn new() -> Grid {
        Grid {
            chunks: Default::default(),
            chunk_slab: Default::default(),
        }
    }

    /// Read a cell of the grid.  Returns 0 for cells which don't contain data.
    pub fn read(&self, x: i64, y: i64) -> u32 {
        let dest = WriteDestination::from_coords(x, y);
        self.chunks
            .get_cached(&dest.chunk)
            .map(|x| self.chunk_slab.get(x).read(dest.x, dest.y))
            .unwrap_or(0)
    }

    /// Write a location in the grid, returning the old value.
    pub fn write(&mut self, x: i64, y: i64, value: u32) -> u32 {
        let dest = WriteDestination::from_coords(x, y);
        // The borrow checker needs help here.
        let chunks_map = &mut self.chunks;
        let chunk_slab = &mut self.chunk_slab;
        let ch = chunks_map.get_or_insert(&dest.chunk, || chunk_slab.insert(Chunk::new(0)));
        chunk_slab.get_mut(ch).write(dest.x, dest.y, value)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

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
            let mut grid = Grid::new();

            for (x, y, val) in tups {
                let good_old = good_impl.insert((x, y), val).unwrap_or(0);
                let old = grid.write(x, y, val);
                prop_assert_eq!(old, good_old);
            }

            // Now check that reading it all back works.
            for ((e_x, e_y), e_val) in good_impl.into_iter() {
                prop_assert_eq!(grid.read(e_x, e_y), e_val);
            }
        }
    }
}
