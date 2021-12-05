use std::sync::Mutex;

use anyhow::Result;
use rand::prelude::*;

use crate::buffer::Buffer;

struct State {
    rng: rand_xorshift::XorShiftRng,
    last: Option<usize>,
}

pub(crate) struct RandomChooser {
    buffers: Vec<Buffer>,
    state: Mutex<State>,
}

impl RandomChooser {
    pub(crate) fn new(buffers: Vec<Buffer>) -> Result<RandomChooser> {
        if buffers.is_empty() {
            anyhow::bail!("Cannot randomly choose from zero buffers");
        }
        Ok(RandomChooser {
            buffers,
            state: Mutex::new(State {
                rng: rand_xorshift::XorShiftRng::from_entropy(),
                last: None,
            }),
        })
    }

    pub(crate) fn choose(&self) -> Buffer {
        // We actually want random, but not reusing the last one we chose.
        loop {
            let mut state = self.state.lock().unwrap();
            let ind = state.rng.gen_range(0..self.buffers.len());
            if self.buffers.len() == 1 || state.last != Some(ind) {
                return self.buffers[ind].clone();
            }
        }
    }
}
