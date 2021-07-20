//! Knows how to go from a pair of i64 coords to a chunk id (the bottom left
//! corner) and an offset in that chunk.
use crate::chunk::*;

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub(crate) struct ChunkId(i64, i64);

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct WriteDestination {
    /// The id of the chunk. Should be first, so that sorting writes will sort by the chunk that they're for.
    pub(crate) chunk: ChunkId,
    /// x coordinate in the chunk.
    pub(crate) x: usize,
    /// y coordinate in the chunk.
    pub(crate) y: usize,
}

impl WriteDestination {
    pub(crate) fn from_coords(x: i64, y: i64) -> WriteDestination {
        // we want to take the floor of the integer with respect to the cell
        // size. To do that, recall that negative integers start at u64::MAX/2
        // and climb toward -1.  This means that by going to u64 and taking the
        // flor, then doing the subtraction, we can get the offset relative to
        // the "bottom" of the cell, so that cells always have the same
        // orientation (i.e. x is always going right, y is always going up,
        // there's no mirroring).
        //
        // This isn't portable to non-twos-complement platforms, which is why we
        // are careful to cover it with unit tests.
        let pos_x = u64::from_ne_bytes(x.to_ne_bytes());
        let pos_y = u64::from_ne_bytes(y.to_ne_bytes());
        let cid = ChunkId(
            i64::from_ne_bytes((pos_x / CHUNK_WIDTH as u64 * CHUNK_WIDTH as u64).to_ne_bytes()),
            i64::from_ne_bytes((pos_y / CHUNK_HEIGHT as u64 * CHUNK_HEIGHT as u64).to_ne_bytes()),
        );
        WriteDestination {
            x: (x - cid.0) as usize,
            y: (y - cid.1) as usize,
            chunk: cid,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_destination_simple() {
        assert_eq!(
            WriteDestination::from_coords(0, 0),
            WriteDestination {
                chunk: ChunkId(0, 0),
                x: 0,
                y: 0
            }
        );
        assert_eq!(
            WriteDestination::from_coords(0, 1),
            WriteDestination {
                chunk: ChunkId(0, 0),
                x: 0,
                y: 1
            }
        );
        assert_eq!(
            WriteDestination::from_coords(5, 5),
            WriteDestination {
                chunk: ChunkId(0, 0),
                x: 5,
                y: 5
            }
        );
        assert_eq!(
            WriteDestination::from_coords(129, 0),
            WriteDestination {
                chunk: ChunkId(128, 0),
                x: 1,
                y: 0
            }
        );
        assert_eq!(
            WriteDestination::from_coords(1030, 129),
            WriteDestination {
                chunk: ChunkId(1024, 128),
                x: 6,
                y: 1
            }
        );
    }

    #[test]
    fn test_write_destination_negative() {
        // Proves that the complicated twos complement stuff worked.
        assert_eq!(
            WriteDestination::from_coords(-7, 0),
            WriteDestination {
                chunk: ChunkId(-128, 0),
                x: 121,
                y: 0
            }
        );
        assert_eq!(
            WriteDestination::from_coords(-7, -129),
            WriteDestination {
                chunk: ChunkId(-128, -256),
                x: 121,
                y: 127
            }
        );
    }
}
