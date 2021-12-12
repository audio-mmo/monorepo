use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use crossbeam::channel as chan;
use log::*;
use rayon::prelude::*;
use synthizer as syz;

use crate::buffer::Buffer;
use crate::io_provider::IoProvider;

/// A pool of threads which decodes buffers upon request.
pub(crate) struct DecodingPool {
    command_sender: chan::Sender<DecodingCommand>,
    pool: rayon::ThreadPool,
    /// When this pool is dropped, we set this flag to true.
    has_dropped: Arc<AtomicBool>,
}

struct DecodingCommand {
    key: Arc<str>,
    result_sender: chan::Sender<Result<Arc<syz::Buffer>>>,
}

impl Drop for DecodingPool {
    fn drop(&mut self) {
        // Signal the thread to stop.
        self.has_dropped.store(true, Ordering::Relaxed);
    }
}

/// the decoding thread.
///
/// This is spawned in the background, reads from the specified channel with the specified concurrency, and stops
/// (failing all decoding requests outstanding) when the flag goes to true.  Assumes it is installed in a properly
/// configured Rayon thread pool.
fn decoding_thread(
    commands: chan::Receiver<DecodingCommand>,
    stop_flag: Arc<AtomicBool>,
    source: Box<dyn IoProvider>,
) {
    info!("Audio decoding threads started");

    commands.iter().par_bridge()
        // First, work out what happens.
    .for_each(|command| {
        let try_block = || -> Result<Arc<syz::Buffer>> {
        if stop_flag.load(Ordering::Relaxed) {
            anyhow::bail!(
                "Decoding for key {} failed because the thread pool was stopped while this request was still outstanding",
                command.key);
            }

            debug!("Decoding asset {}", command.key);
            let start = Instant::now();
            let buffer = source.decode_buffer(&command.key)?;
            let end = Instant::now();
            debug!("Decoded {} in {} seconds", command.key, (end-start).as_secs_f64());
            Ok(buffer)
        };

        let _ = command.result_sender.send(try_block());
        });
}

impl DecodingPool {
    pub(crate) fn new(
        concurrency: usize,
        channel_len: usize,
        source: Box<dyn IoProvider>,
    ) -> Result<DecodingPool> {
        let has_dropped = Arc::new(AtomicBool::new(false));
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(concurrency)
            .thread_name(|n| format!("Decoding thread {}", n))
            .build()?;
        let (command_sender, command_receiver) = chan::bounded(channel_len);
        let cloned_flag = has_dropped.clone();
        pool.spawn(move || {
            decoding_thread(command_receiver, cloned_flag, source);
        });

        Ok(DecodingPool {
            command_sender,
            has_dropped,
            pool,
        })
    }

    pub(crate) fn decode(&self, key: Arc<str>) -> Result<Buffer> {
        let (sender, receiver) = chan::bounded(1);
        let command = DecodingCommand {
            result_sender: sender,
            key,
        };

        self.command_sender.send(command)?;
        Ok(Buffer::new_decoding(receiver))
    }
}
