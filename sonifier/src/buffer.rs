use std::sync::{Arc, RwLock};

use anyhow::Result;
use crossbeam::channel as chan;
use synthizer as syz;

use crate::Engine;

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
///
/// These are internally reference counted. Clone creates a second buffer referencing the same data.
#[derive(Clone)]
pub struct Buffer {
    state: Arc<RwLock<BufferState>>,
}

impl Buffer {
    pub(crate) fn new_decoding(receiver: chan::Receiver<Result<Arc<syz::Buffer>>>) -> Buffer {
        Buffer {
            state: Arc::new(RwLock::new(BufferState::Decoding { receiver })),
        }
    }

    fn await_decoding_finished(&self) -> Result<Arc<syz::Buffer>> {
        // First try the read-side, as this is the common case.
        {
            let guard = self.state.read().unwrap();
            match &*guard {
                BufferState::Decoded { ref buffer } => return Ok(buffer.clone()),
                BufferState::Failed => anyhow::bail!("This buffer failed to decode"),
                _ => {}
            }
        }

        let mut guard = self.state.write().unwrap();
        let (newstate, res): (Option<BufferState>, Arc<syz::Buffer>) = match *guard {
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
            *guard = s;
        }
        Ok(res)
    }

    /// get the Synthizer buffer, possibly blocking if decoding is still in progress.
    pub(crate) fn as_synthizer(&mut self) -> Result<Arc<syz::Buffer>> {
        self.await_decoding_finished()
    }
}

/// A reference-counted handle to an audio buffer.
#[derive(Clone)]
pub struct BufferHandle(Arc<Engine>, Arc<Buffer>);
