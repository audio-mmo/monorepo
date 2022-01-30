use std::marker::Unpin;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex, Weak,
};
use std::time::Duration;

use anyhow::Result;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::Notify;
use tokio::time::Instant;

use ammo_framer::{Framer, Message, Parser, ParserOutcome};

use crate::authentication::*;
use crate::connection::*;

const PARSER_CAP: usize = 8192;
const READ_BUF_SIZE: usize = 8192;
const WRITE_BUF_SIZE: usize = 8192;

#[derive(derivative::Derivative, Debug, Clone)]
#[derivative(Default)]
pub struct NetworkConnectionConfig {
    #[derivative(Default(value = "8192"))]
    max_auth_message_size: usize,

    #[derivative(Default(value = "Duration::from_millis(500)"))]
    auth_message_timeout: Duration,

    /// Maximum number of pendin bytes which may be unsent before a connection should be shut down.
    ///
    /// When exceeded, the connection ungracefully closes.
    #[derivative(Default(value = "1<<20"))]
    max_unsent_bytes: usize,

    /// This interval specifies the maximum amount of time we are willing to go without seeing a message before giving
    /// up and shutting down.
    ///
    /// Since there are regular ticks a few times a second, this should usually be on the order of a second to dela with
    /// network hiccups; anything longer than that and the player is playing on a connection that won't give results no
    /// matter what we do, or the server is in a situation where we have much bigger problems.
    ///
    /// The implementation of this relies on the consumer of the connection to read messages promptly: we detect that we
    /// had a complete message using the last time a message was decoded.
    #[derivative(Default(value = "Duration::from_secs(1)"))]
    max_message_interval: Duration,

    /// Max length of an incoming message.
    max_incoming_message_length: Option<u64>,

    /// Timeout on individual write calls.
    #[derivative(Default(value = "Duration::from_millis(500)"))]
    write_timeout: Duration,
}

pub(crate) struct NetworkConnection {
    config: NetworkConnectionConfig,
    authenticator: Option<Arc<dyn Authenticator>>,
    close_notifier: Notify,
    framer: Mutex<Framer>,
    parser: Mutex<Parser>,

    /// Number of messages decoded from this connection so far.
    decoded_messages: AtomicU64,

    /// True after we have the authentication message; false if the connection's task ends, whether successfully or
    /// otherwise.
    connected: AtomicBool,
}

/// We hold a weak reference, so that the connection goes away when either the task dies or the handle dies, whichever
/// comes first.
struct NetworkConnectionHandle(Weak<NetworkConnection>);

impl NetworkConnection {
    pub(crate) fn new(
        config: NetworkConnectionConfig,
        authenticator: Option<Arc<dyn Authenticator>>,
    ) -> NetworkConnection {
        NetworkConnection {
            authenticator,
            close_notifier: Notify::new(),
            framer: Mutex::new(Framer::new()),
            parser: Mutex::new(Parser::new(config.max_incoming_message_length, PARSER_CAP)),
            config,
            decoded_messages: AtomicU64::new(0),
            connected: AtomicBool::new(false),
        }
    }

    pub(crate) async fn task(
        self,
        transport: impl AsyncRead + AsyncWrite,
        permit: Option<tokio::sync::OwnedSemaphorePermit>,
    ) -> Result<()> {
        let aself = Arc::new(self);
        let res = aself.task_inner(transport, permit).await;
        aself.connected.store(false, Ordering::Relaxed);
        res
    }

    async fn task_inner(
        self: &Arc<Self>,
        transport: impl AsyncRead + AsyncWrite,
        _permit: Option<tokio::sync::OwnedSemaphorePermit>,
    ) -> Result<()> {
        let mut read_buf: [u8; READ_BUF_SIZE] = [0; READ_BUF_SIZE];
        let mut write_buf: [u8; WRITE_BUF_SIZE] = [0; WRITE_BUF_SIZE];
        let mut write_buf_size = 0;
        let mut write_buf_cursor = 0;

        let (mut reader, mut writer) = tokio::io::split(transport);
        if let Some(authenticator) = self.authenticator.as_ref() {
            // Before anything else, we read the first auth message.
            let mut auth_bytes = 0;
            let auth_deadline = Instant::now() + self.config.auth_message_timeout;
            while auth_bytes < self.config.max_auth_message_size {
                let can_read = read_buf
                    .len()
                    .min(self.config.max_auth_message_size - auth_bytes);
                let buf_slice = &mut read_buf[0..can_read];
                tokio::select! {
                    maybe_got = reader.read(buf_slice) => {
                        let got = maybe_got?;
                        auth_bytes += got;
                        let mut parser = self.parser.lock().unwrap();
                        parser.feed(&mut &buf_slice[..got])?;
                        if let ParserOutcome::Message(m) = parser.read_message()? {
                            self.connected.store(true, Ordering::Relaxed);
                            let handle = Arc::new(NetworkConnectionHandle(Arc::downgrade(self)));
                            authenticator.authenticate(&m, handle)?;
                            parser.roll_forward()?;

                        }
                    },
                    _ =  tokio::time::sleep_until(auth_deadline) => {
                        anyhow::bail!("Timeout waiting for auth message");
                    },
                }
            }
        }

        // We need to start off by filling the write buffer ourselves.

        let mut message_deadline = Instant::now() + self.config.max_message_interval;
        let mut decoded_messages = self.decoded_messages.load(Ordering::Relaxed);

        loop {
            if write_buf_cursor == write_buf_size {
                let guard = self.framer.lock().unwrap();
                let front = guard.read_front(write_buf.len());
                let front_len = front.len();
                write_buf[..front_len].copy_from_slice(front);
                write_buf_size = front.len();
            }

            tokio::select! {
                maybe_got = reader.read(&mut read_buf[..]) => {
                    let got = maybe_got?;
                    self.parser.lock().unwrap().feed(&mut &read_buf[..got])?;
                },
                maybe_wrote = tokio::time::timeout(self.config.write_timeout, writer.write(&write_buf[write_buf_cursor..write_buf_size]))  => {
                    let wrote = maybe_wrote??;
                    if wrote == 0 {
                        // EOF; nothing left to do.
                        return Ok(());
                    }

                    write_buf_cursor += wrote;
                },
                _ = tokio::time::sleep_until(message_deadline) => {
                    let new_decoded_messages = self.decoded_messages.load(Ordering::Relaxed);
                    if new_decoded_messages == decoded_messages {
                        anyhow::bail!("Received messages too slowly");
                    }
                    decoded_messages = new_decoded_messages;
                    message_deadline = Instant::now() + self.config.max_message_interval;
                },
                _ = self.close_notifier.notified() => {
                    return Ok(());
                }
            }

            // Check this down here.  We want at least one read to be able to go through.
            if self.framer.lock().unwrap().pending_bytes() > self.config.max_unsent_bytes {
                anyhow::bail!("Too many outstanding bytes");
            }
        }
    }
}

impl Drop for NetworkConnectionHandle {
    fn drop(&mut self) {
        if let Some(x) = self.0.upgrade() {
            x.close_notifier.notify_one()
        }
    }
}

impl NetworkConnectionHandle {
    fn with_good_conn<R>(&self, cb: impl FnOnce(&NetworkConnection) -> Result<R>) -> Result<R> {
        let strong = self
            .0
            .upgrade()
            .ok_or_else(|| anyhow::anyhow!("Connection task is dead"))?;
        cb(&*strong)
    }
}

impl Connection for NetworkConnectionHandle {
    fn receive_messages(&self, callback: &mut dyn FnMut(&Message) -> Result<()>) -> Result<()> {
        self.with_good_conn(|conn| {
            let mut parser = conn.parser.lock().unwrap();
            while let ParserOutcome::Message(m) = parser.read_message()? {
                callback(&m)?;
                parser.roll_forward()?;
            }
            Ok(())
        })
    }

    fn send_message(&self, message: &Message) -> Result<()> {
        self.with_good_conn(|c| {
            let mut framer = c.framer.lock().unwrap();
            framer.add_message(message);
            Ok(())
        })
    }

    fn is_connected(&self) -> bool {
        self.with_good_conn(|c| Ok(c.connected.load(Ordering::Relaxed)))
            .unwrap_or(false)
    }
}
