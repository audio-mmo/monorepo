use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tokio::io::{AsyncRead, AsyncWrite};

use ammo_framer::Message;

use crate::message_handling::*;

#[derive(derivative::Derivative, Debug, Clone)]
#[derivative(Default)]
pub struct ConnectionConfig {
    /// Maximum number of pendin bytes which may be unsent before a connection should be shut down.
    ///
    /// When exceeded, the connection ungracefully closes.
    #[derivative(Default(value = "1<<20"))]
    max_unsent_bytes: usize,

    /// This interval specifies the maximum amount of time we are willing to go without seeing a message before giving
    /// up and shutting down.
    ///
    /// Since there are regular ticks a few times a second, this should uisually be on the order of a second to dela
    /// with network hiccups; anything longer than that and the player is playing on a connection that won't give
    /// results no matter what we do.
    #[derivative(Default(value = "Duration::from_secs(1)"))]
    max_message_interval: Duration,

    /// Timeout on individual write calls.
    #[derivative(Default(value = "Duration::from_millis(500)"))]
    write_timeout: Duration,
}

pub(crate) struct Connection<F, NT> {
    config: ConnectionConfig,
    message_handler_factory: Arc<F>,
    net_transport: NT,
}

impl<F: MessageHandlerFactory, NT: AsyncRead + AsyncWrite> Connection<F, NT> {
    pub(crate) fn new(
        config: ConnectionConfig,
        message_handler_factory: F,
        net_transport: NT,
    ) -> Connection<F, NT> {
        Connection {
            config,
            message_handler_factory: Arc::new(message_handler_factory),
            net_transport,
        }
    }

    /// Consume this connection, spawning it into the current runtime.
    ///
    /// The optional semaphore permit is used to implement rate limiting if this is a server connection.
    pub(crate) fn spawn_consume(
        self,
        _permit: Option<tokio::sync::OwnedSemaphorePermit>,
    ) -> Result<()> {
        Ok(())
    }
}
