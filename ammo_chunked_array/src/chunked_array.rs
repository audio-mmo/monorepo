//! A chunked array consists of some number of chunks behind a hashmap of chunk
//! coordinates, plus a lookup table that can translate to and/or from `T`.
//!
//! For convenience with game maps, this implementation supports negative
//! indices.  Note however that the array doesn't back itself on disk, and so
//! using more space than machine memory will crash.
//!
//! In the name of expediency the current implementation doesn't compress.

use crate::chunks::*;

#[derive(Copy, Clone, Eq, Hash, PartialEq, PartialOrd, Ord)]
struct ChunkId(u64, u64);

struct WriteDestination {
    chunk: ChunkId,
    /// x coordinate in the chunk.
    x: usize,
    /// y coordinate in the chunk.
    y: usize,
}

impl WriteDestination {
    fn from_coords(x: i64, y: i64) -> WriteDestination {
        // we want to take the floor of the integer with respect to the cell
        // size. To do that, recall that negative integers start at u64::MAX/2
        // and climb toward -1.  This means that by going to u64 and taking the
        // flor, then doing the subtraction, we can get the offset relative to
        // the "bottom" of the cell, so that cells always have the same
        // orientation (i.e. x is always going right, y is always going up,
        // there's no mirroring).
        //
        // This isn't portable to non-twos-complement platforms.
        let pos_x = u64::from_ne_bytes(x.to_ne_bytes());
        let pos_y = u64::from_ne_bytes(y.to_ne_bytes());
        let cid = ChunkId(
            pos_x / CHUNK_WIDTH as u64 * CHUNK_WIDTH as u64,
            pos_y / CHUNK_HEIGHT as u64 * CHUNK_HEIGHT as u64,
        );
        WriteDestination {
            x: (pos_x - cid.0) as usize,
            y: (pos_y - cid.0) as usize,
            chunk: cid,
        }
    }
}
