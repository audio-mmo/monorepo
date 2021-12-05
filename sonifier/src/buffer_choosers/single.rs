use anyhow::Result;

use crate::buffer::Buffer;

pub(crate) struct SingleChooser {
    buffer: Buffer,
}

impl SingleChooser {
    pub(crate) fn new(buffer: Buffer) -> Result<SingleChooser> {
        Ok(SingleChooser { buffer })
    }

    pub(crate) fn choose(&self) -> Buffer {
        self.buffer.clone()
    }
}
