pub(crate) mod random;
pub(crate) mod round_robin;
pub(crate) mod single;

use anyhow::Result;

use crate::buffer::Buffer;

use random::*;
use round_robin::*;
use single::*;

// Need to put this in a struct to hide the variants.
enum BufferChooserInner {
    Single(SingleChooser),
    RoundRobin(RoundRobinChooser),
    Random(RandomChooser),
}

pub struct BufferChooser {
    inner: BufferChooserInner,
}

impl BufferChooser {
    pub fn new_single(buffer: Buffer) -> Result<BufferChooser> {
        let chooser = SingleChooser::new(buffer)?;
        Ok(BufferChooser {
            inner: BufferChooserInner::Single(chooser),
        })
    }

    pub fn new_random(buffers: Vec<Buffer>) -> Result<BufferChooser> {
        let chooser = RandomChooser::new(buffers)?;
        Ok(BufferChooser {
            inner: BufferChooserInner::Random(chooser),
        })
    }

    pub fn new_round_robin(buffers: Vec<Buffer>) -> Result<BufferChooser> {
        let chooser = RoundRobinChooser::new(buffers)?;
        Ok(BufferChooser {
            inner: BufferChooserInner::RoundRobin(chooser),
        })
    }
}
