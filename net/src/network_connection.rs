use std::marker::Unpin;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

use anyhow::Result;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::Notify;
use tokio::time::Instant;

use ammo_framer::{Framer, Message, Parser, ParserOutcome};

use crate::authentication::*;

const FRAMER_CAP: usize = 8192;
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

pub(crate) struct NetworkConnection<NT> {
    config: NetworkConnectionConfig,
    authenticator: Arc<dyn Authenticator>,
    net_transport: NT,
    drop_notifier: Notify,
    framer: Mutex<Framer>,
    parser: Mutex<Parser>,

    /// Number of messages decoded from this connection so far.
    decoded_messages: AtomicU64,
}

impl<NT: AsyncRead + AsyncWrite + Unpin> NetworkConnection<NT> {
    pub(crate) fn new(
        config: NetworkConnectionConfig,
        authenticator: Arc<dyn Authenticator>,
        net_transport: NT,
    ) -> NetworkConnection<NT> {
        NetworkConnection {
            authenticator,
            net_transport,
            drop_notifier: Notify::new(),
            framer: Mutex::new(Framer::new(FRAMER_CAP)),
            parser: Mutex::new(Parser::new(config.max_incoming_message_length, PARSER_CAP)),
            config,
            decoded_messages: AtomicU64::new(0),
        }
    }

    pub(crate) async fn task(
        mut self,
        _permit: Option<tokio::sync::OwnedSemaphorePermit>,
    ) -> Result<()> {
        let mut read_buf: [u8; READ_BUF_SIZE] = [0; READ_BUF_SIZE];
        let mut write_buf: [u8; WRITE_BUF_SIZE] = [0; WRITE_BUF_SIZE];

        // Before anything else, we read the first auth message.

        let mut auth_bytes = 0;
        let auth_deadline = Instant::now() + self.config.auth_message_timeout;
        while auth_bytes < self.config.max_auth_message_size {
            let can_read = read_buf
                .len()
                .min(self.config.max_auth_message_size - auth_bytes);
            let buf_slice = &mut read_buf[0..can_read];
            tokio::select! {
                maybe_got = self.net_transport.read(buf_slice) => {
                    let got = maybe_got?;
                    auth_bytes += got;
                    let mut parser = self.parser.lock().unwrap();
                    parser.feed(&mut &buf_slice[..got])?;
                    if let ParserOutcome::Message(_) = parser.read_message()? {
                        parser.roll_forward()?;
                        todo!();
                    }
                },
                _ =  tokio::time::sleep_until(auth_deadline) => {
                    anyhow::bail!("Timeout waiting for auth message");
                },
            }
        }

        let mut message_deadline = Instant::now() + self.config.max_message_interval;
        let mut decoded_messages = self.decoded_messages.load(Ordering::Relaxed);

        loop {}

        Ok(())
    }
}
