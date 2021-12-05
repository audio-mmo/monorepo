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

/// Chooses a buffer using various strategies.
///
/// In practice, we frequently don't want just one buffer.  Take for example the case of footsteps, for which we want to
/// actually choose randomly from a list.  This type abstracts over the operation of choosing a buffer from a given
/// strategy.
///
/// This is threadsafe. It is possible to share the same chooser between multiple objects, or even pull on it yourself
/// from multiple threads.
pub struct BufferChooser {
    inner: BufferChooserInner,
}

impl BufferChooser {
    /// Create a chooser which simply returns the single buffer.
    pub fn new_single(buffer: Buffer) -> Result<BufferChooser> {
        let chooser = SingleChooser::new(buffer)?;
        Ok(BufferChooser {
            inner: BufferChooserInner::Single(chooser),
        })
    }

    /// Create a chooser which selects randomly.
    ///
    /// In practice it's actually random plus some tweaks to avoid it not sounding random.  We don't want the same
    /// buffer happening twice in a row for example.
    pub fn new_random(buffers: Vec<Buffer>) -> Result<BufferChooser> {
        let chooser = RandomChooser::new(buffers)?;
        Ok(BufferChooser {
            inner: BufferChooserInner::Random(chooser),
        })
    }

    /// Create a chooser which starts at the first buffer and selects in a loop before returning to the beginning of the
    /// choices.
    pub fn new_round_robin(buffers: Vec<Buffer>) -> Result<BufferChooser> {
        let chooser = RoundRobinChooser::new(buffers)?;
        Ok(BufferChooser {
            inner: BufferChooserInner::RoundRobin(chooser),
        })
    }
}
