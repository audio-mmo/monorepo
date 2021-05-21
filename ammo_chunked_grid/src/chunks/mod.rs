//! Chunks are encoded as a series of m*n boxes which are allocated inline on demand.

// We need these constants because Rust const generics isn't yet powerful enough
// to be abstract over the dimensions of the cells.
///
/// Should evenly divide u64::MAX.
pub(crate) const CHUNK_WIDTH: usize = 128;
pub(crate) const CHUNK_HEIGHT: usize = 128;
pub(crate) const BOX_WIDTH: usize = 2;
pub(crate) const BOX_HEIGHT: usize = 2;
const TOTAL_BOXES: usize = CHUNK_WIDTH * CHUNK_HEIGHT / BOX_WIDTH / BOX_HEIGHT;

pub(crate) struct Chunk {
    /// The data of the chunk. Subdivided into small `BOX_WIDTH * BOX_HEIGHT`
    /// subarrays, but with the first index always being the default value of
    /// the chunk, so that looking up boxes through the indirection table can be
    /// branchless.
    data: Vec<u32>,
    /// The offsets of the boxes, using `0` as an unallocated box.
    box_offsets: [u16; TOTAL_BOXES],
}

#[derive(Debug)]
struct BoxRef {
    /// Index of the box in the lookup table.
    box_index: usize,
    /// Index to read/write relative to the box's start for given coordinates.
    data_index: usize,
}

fn coords_to_box(x: usize, y: usize) -> BoxRef {
    let linear_index = y * CHUNK_WIDTH + x;
    let box_index = linear_index / (BOX_WIDTH * BOX_HEIGHT);
    let data_index = linear_index - box_index * BOX_WIDTH * BOX_HEIGHT;
    BoxRef {
        box_index,
        data_index,
    }
}

impl Chunk {
    pub(crate) fn new(default_val: u32) -> Chunk {
        Chunk {
            data: vec![default_val],
            box_offsets: [0; TOTAL_BOXES],
        }
    }

    pub(crate) fn read(&self, x: usize, y: usize) -> u32 {
        let box_data = coords_to_box(x, y);
        let arr_off = self.box_offsets[box_data.box_index];
        self.data[arr_off as usize + box_data.data_index]
    }

    /// Write to the cell, returning the old value.
    pub(crate) fn write(&mut self, x: usize, y: usize, value: u32) -> u32 {
        let box_info = coords_to_box(x, y);
        //println!("{:?}", box_info);
        let defval = self.data[0];
        let mut box_ind = self.box_offsets[box_info.box_index];
        if box_ind == 0 {
            self.data
                .resize(self.data.len() + BOX_WIDTH * BOX_HEIGHT, defval);
            box_ind = (self.data.len() - 4) as u16;
            self.box_offsets[box_info.box_index] = box_ind;
        }
        let dest = &mut self.data[box_ind as usize + box_info.data_index];
        let old = *dest;
        *dest = value;
        old
    }
}
