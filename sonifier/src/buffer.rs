use std::cell::RefCell;
use std::sync::Arc;

use anyhow::Result;
use crossbeam::channel as chan;
use synthizer as syz;

/// Internal state for a buffer.
enum BufferState {
    /// This buffer is decoding, which means that it is enqueued with the decoding pool.
    ///
    /// At some point in the future, the channel will get the result of decoding.
    Decoding {
        receiver: chan::Receiver<Result<Arc<syz::Buffer>>>,
    },
    /// This buffer is decoded successfully.
    Decoded { buffer: Arc<syz::Buffer> },
    /// This buffer failed to decode.
    ///
    /// The error is communicated out the first time the buffer is used.
    Failed,
}

/// A buffer.  This is created from the decoding pool.
pub(crate) struct Buffer {
    state: RefCell<BufferState>,
}

impl Buffer {
    pub(crate) fn new_decoding(receiver: chan::Receiver<Result<Arc<syz::Buffer>>>) -> Buffer {
        Buffer {
            state: RefCell::new(BufferState::Decoding { receiver }),
        }
    }

    fn await_decoding_finished(&self) -> Result<Arc<syz::Buffer>> {
        let (newstate, res): (Option<BufferState>, Arc<syz::Buffer>) = match &*self.state.borrow() {
            BufferState::Decoded { ref buffer } => (None, buffer.clone()),
            BufferState::Failed => {
                anyhow::bail!("This buffer already failed to decode. Not trying again");
            }
            BufferState::Decoding { ref receiver } => {
                let buffer = receiver.recv()??;
                (
                    Some(BufferState::Decoded {
                        buffer: buffer.clone(),
                    }),
                    buffer,
                )
            }
        };

        if let Some(s) = newstate {
            self.state.replace(s);
        }
        Ok(res)
    }

    /// get the Synthizer buffer, possibly blocking if decoding is still in progress.
    pub(crate) fn as_synthizer(&mut self) -> Result<Arc<syz::Buffer>> {
        self.await_decoding_finished()
    }
}
