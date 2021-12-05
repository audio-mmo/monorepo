use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::Result;

use crate::buffer::Buffer;

pub(crate) struct RoundRobinChooser {
    buffers: Vec<Buffer>,
    /// See comments in choose for why we use AtomicU64 instead of AtomicUsize.
    counter: AtomicU64,
}

impl RoundRobinChooser {
    pub(crate) fn new(buffers: Vec<Buffer>) -> Result<RoundRobinChooser> {
        if buffers.is_empty() {
            anyhow::bail!("Cannot round robin zero buffers");
        }

        Ok(RoundRobinChooser {
            buffers,
            counter: AtomicU64::new(0),
        })
    }

    pub(crate) fn choose(&self) -> Buffer {
        // We use AtomicU64 because we can increment it forever without having to worry about wraparound or CAS loops.
        // So just grab the forever-increasing counter and take the remainder.
        let index = self.counter.fetch_add(1, Ordering::Relaxed) % self.buffers.len() as u64;
        self.buffers[index as usize].clone()
    }
}
