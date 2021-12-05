use std::sync::Mutex;

use anyhow::Result;
use rand::prelude::*;

use crate::buffer::Buffer;

pub(crate) struct RandomChooser {
    buffers: Vec<Buffer>,
    rng: Mutex<rand_xorshift::XorShiftRng>,
}

impl RandomChooser {
    pub(crate) fn new(buffers: Vec<Buffer>) -> Result<RandomChooser> {
        if buffers.is_empty() {
            anyhow::bail!("Cannot randomly choose from zero buffers");
        }
        Ok(RandomChooser {
            buffers,
            rng: Mutex::new(rand_xorshift::XorShiftRng::from_entropy()),
        })
    }

    pub(crate) fn choose(&self) -> Buffer {
        let ind = self.rng.lock().unwrap().gen_range(0..self.buffers.len());
        self.buffers[ind].clone()
    }
}
