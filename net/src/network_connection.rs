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

use crate::connection::*;

const PARSER_CAP: usize = 8192;
const READ_BUF_SIZE: usize = 8192;
const WRITE_BUF_SIZE: usize = 8192;

#[derive(derivative::Derivative, Debug, Clone)]
#[derivative(Default)]
pub struct NetworkConnectionConfig {
    /// Maximum length of the first message.  This is usually used for authentication/handshaking.
    #[derivative(Default(value = "8192"))]
    first_message_max_len: usize,

    /// Timeout within which we must receive the first message.
    #[derivative(Default(value = "Duration::from_secs(1)"))]
    first_message_timeout: Duration,

    /// Maximum number of pendin bytes which may be unsent before a connection should be shut down.
    ///
    /// When exceeded, the connection ungracefully closes.
    #[derivative(Default(value = "1<<20"))]
    max_unsent_bytes: usize,

    #[derivative(Default(value = "1<<30"))]
    max_unparsed_bytes: usize,

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

    /// How long the connection is allowed to shut down for before we give up sending any outstanding data.
    #[derivative(Default(value = "Duration::from_secs(5)"))]
    shutdown_timeout: Duration,
}

pub(crate) struct NetworkConnection {
    config: NetworkConnectionConfig,
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
    pub(crate) fn new(config: NetworkConnectionConfig) -> NetworkConnection {
        NetworkConnection {
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
        return_channel: tokio::sync::mpsc::Sender<Arc<dyn Connection>>,
    ) -> Result<()> {
        let aself = Arc::new(self);
        let res = aself.task_inner(transport, return_channel).await;
        aself.connected.store(false, Ordering::Relaxed);

        // Let's be explicit about this, for clarity.
        std::mem::drop(permit);

        res
    }

    async fn task_inner(
        self: &Arc<Self>,
        transport: impl AsyncRead + AsyncWrite,
        return_channel: tokio::sync::mpsc::Sender<Arc<dyn Connection>>,
    ) -> Result<()> {
        let mut read_buf: [u8; READ_BUF_SIZE] = [0; READ_BUF_SIZE];
        let mut write_buf: [u8; WRITE_BUF_SIZE] = [0; WRITE_BUF_SIZE];
        let mut write_buf_size = 0;
        let mut write_buf_cursor = 0;

        let (mut reader, mut writer) = tokio::io::split(transport);
        let first_msg_deadline = tokio::time::sleep(self.config.first_message_timeout);
        tokio::pin!(first_msg_deadline);

        loop {
            tokio::select! {
                maybe_read = reader.read(&mut read_buf[..]) => {
                    let read = maybe_read?;
                    if read == 0 {
                        return Ok(());
                    }

                    self.parser.lock().unwrap().feed(&mut &read_buf[..read])?;
                },
                _ = &mut first_msg_deadline => {
                    anyhow::bail!("Took too long to read the first message");
                }
            }

            let parser = self.parser.lock().unwrap();
            if let ParserOutcome::Message(_) = parser.read_message()? {
                // We have at least one message.
                break;
            }

            if parser.contained_bytes() > self.config.first_message_max_len {
                anyhow::bail!("First message too long");
            }
        }

        let handle = Arc::new(NetworkConnectionHandle(Arc::downgrade(self)));
        return_channel.send(handle).await.map_err(|_| {
            anyhow::anyhow!("Unable to send cionnection handle because the faar side hung up")
        })?;

        let mut decoded_messages = self.decoded_messages.load(Ordering::Relaxed);
        let mut message_deadline_interval = tokio::time::interval(self.config.max_message_interval);
        message_deadline_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        // The first tick completes immediately.
        message_deadline_interval.tick().await;

        loop {
            if write_buf_cursor == write_buf_size {
                let guard = self.framer.lock().unwrap();
                let front = guard.read_front(write_buf.len());
                let front_len = front.len();
                write_buf[..front_len].copy_from_slice(front);
                write_buf_size = front.len();
            }

            let can_read =
                self.parser.lock().unwrap().contained_bytes() < self.config.max_unparsed_bytes;
            let can_write = write_buf_cursor < write_buf_size;

            tokio::select! {
                maybe_got = reader.read(&mut read_buf[..]), if can_read  => {
                    let got = maybe_got?;
                    self.parser.lock().unwrap().feed(&mut &read_buf[..got])?;
                },
                maybe_wrote = (tokio::time::timeout(self.config.write_timeout, writer.write(&write_buf[write_buf_cursor..write_buf_size]))),
                    if can_write => {
                    let wrote = maybe_wrote??;
                    if wrote == 0 {
                        // EOF, which means that the other side closed the connection.  In this case, don't try to send
                        // the remaining bytes; there's nothing there to listen for them.
                        return Ok(());
                    }

                    write_buf_cursor += wrote;
                },
                _ = message_deadline_interval.tick() => {
                    let new_decoded_messages = self.decoded_messages.load(Ordering::Relaxed);
                    if new_decoded_messages == decoded_messages {
                        anyhow::bail!("Received messages too slowly");
                    }
                    decoded_messages = new_decoded_messages;
                },
                _ = self.close_notifier.notified() => {
                    // We still want to try to drain the framer.
                    break;
                }
            }

            // Check this down here.  We want at least one read to be able to go through.
            if self.framer.lock().unwrap().pending_bytes() > self.config.max_unsent_bytes {
                anyhow::bail!("Too many outstanding bytes");
            }
        }

        // Mark this connection as no longer connected. This ensures that no more messages can be sent and as a
        // consequence nothing blocks on the mutex anymore.
        self.connected.store(false, Ordering::Relaxed);

        // We must steal the framer's data because we can't hold the mutex past an await point.
        let framer = self.framer.lock().unwrap().steal();
        match tokio::time::timeout(
            self.config.shutdown_timeout,
            writer.write_all(framer.read_front(framer.pending_bytes())),
        )
        .await
        {
            Err(_) => {
                // Not an error if we timed out.
                return Ok(());
            }
            Ok(x) => x?,
        }

        Ok(())
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
        if !strong.connected.load(Ordering::Relaxed) {
            anyhow::bail!("Connection is no longer connected");
        }

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
