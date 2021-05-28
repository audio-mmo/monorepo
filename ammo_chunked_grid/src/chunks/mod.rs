//! Chunks are encoded as a series of small subarrays allocated on demand in a vec, e.g.
//! `[[a, b, c, d], [a, b, c, d]]`, which is indexed with a linear index computed by converting
//! `(x, y)` pairs using `y * CHUNK_WIDTH + x`.  The nested subarrays are looked up using an indirection table, so that they can be allocated on demand and so
//! that we can reserve a default value at the beginning of the vec.  This gives a branchless read.  It also gives a
//?! mostly branchless write, save that write needs to check whether or not the subarray in question
//! is pointing at the default value or a writable subarray.

// We need these constants because Rust const generics isn't yet powerful enough
// to be abstract over the dimensions of the cells.
///
/// Should evenly divide u64::MAX.
pub(crate) const CHUNK_WIDTH: usize = 128;
pub(crate) const CHUNK_HEIGHT: usize = 128;
/// Should evenly divide the chunk.
const SUBARRAY_LEN: usize = 4;
const TOTAL_SUBARRAYS: usize = CHUNK_WIDTH * CHUNK_HEIGHT / SUBARRAY_LEN;

pub(crate) struct Chunk {
    /// Data of the chunk.  The first index is the default value for the chunk.
    /// All indices after that are the subarrays in groups of `SUBARRAY_LEN`.
    /// Put another way, the vec is always `1 + SUBARRAY_LEN * n` elements long.
    data: Vec<u32>,
    /// the offsets of the subarrays, using 0 as a sentinel value that instead means default value.
    subarray_offsets: [u16; TOTAL_SUBARRAYS],
}

#[derive(Debug)]
struct SubarrayRef {
    /// Index of the subarray in the lookup table.
    subarray_index: usize,
    /// Index to read/write relative to the subarray's start.
    data_index: usize,
}

fn coords_to_subarray(x: usize, y: usize) -> SubarrayRef {
    let linear_index = y * CHUNK_WIDTH + x;
    let subarray_index = linear_index / SUBARRAY_LEN;
    let data_index = linear_index - subarray_index * SUBARRAY_LEN;
    SubarrayRef {
        subarray_index,
        data_index,
    }
}

impl Chunk {
    pub(crate) fn new(default_val: u32) -> Chunk {
        Chunk {
            data: vec![default_val],
            subarray_offsets: [0; TOTAL_SUBARRAYS],
        }
    }

    pub(crate) fn read(&self, x: usize, y: usize) -> u32 {
        let arr_data = coords_to_subarray(x, y);
        let arr_off = self.subarray_offsets[arr_data.subarray_index];
        // We need to account for subarrays that point at defaults and don't want
        // branches. Let's convert to an int between 0 and 1 and multiply.
        let mul = (arr_off != 0) as usize;
        self.data[arr_off as usize + arr_data.data_index * mul]
    }

    /// Write to the cell, returning the old value.
    pub(crate) fn write(&mut self, x: usize, y: usize, value: u32) -> u32 {
        let arr_info = coords_to_subarray(x, y);
        let defval = self.data[0];
        let mut arr_ind = self.subarray_offsets[arr_info.subarray_index];
        if arr_ind == 0 {
            self.data.resize(self.data.len() + SUBARRAY_LEN, defval);
            arr_ind = (self.data.len() - SUBARRAY_LEN) as u16;
            self.subarray_offsets[arr_info.subarray_index] = arr_ind;
        }
        let dest = &mut self.data[arr_ind as usize + arr_info.data_index];
        let old = *dest;
        *dest = value;
        old
    }
}
