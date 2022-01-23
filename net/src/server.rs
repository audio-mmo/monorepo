use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tokio::net::TcpListener;
use tokio::sync::{mpsc::UnboundedReceiver, Semaphore};

use crate::connection::Connection;
use crate::message_handling::*;

#[derive(Clone, Debug, derive_builder::Builder)]
pub struct ServerConfig {
    interface: std::net::SocketAddr,
    port: usize,

    /// Maximum number of connections which may connect to the server at any one time.
    ///
    /// We start refusing connections if this many are open.
    #[builder(default = "512")]
    max_connections: usize,

    #[builder(default = "Default::default()")]
    connection_config: crate::connection::ConnectionConfig,
}

/// A server.
///
/// This creates [MessageHandler]s and drives them via a given [MessageHandlerFactory].
///
/// We don't support clean shutdown of the server itself for now.
pub struct Server<T> {
    pub(crate) conn_sem: Arc<Semaphore>,
    pub(crate) config: ServerConfig,
    pub(crate) message_handler_factory: Arc<T>,
}

impl<T: MessageHandlerFactory + 'static> Server<T> {
    pub fn new(config: ServerConfig, message_handler_factory: T) -> Arc<Server<T>> {
        Arc::new(Server {
            conn_sem: Arc::new(Semaphore::new(config.max_connections)),
            message_handler_factory: Arc::new(message_handler_factory),
            config,
        })
    }

    /// Drive the server's listening loop ona  Tokio runtime.
    pub async fn listening_loop(self: Arc<Self>) -> Result<()> {
        let listener = TcpListener::bind(self.config.interface).await?;
        loop {
            let permit = self.conn_sem.clone().acquire_owned().await?;
            let (stream, _) = listener.accept().await?;
            let conn = Connection::new(
                self.config.connection_config.clone(),
                self.message_handler_factory.clone(),
                stream,
            );
            conn.spawn_consume(Some(permit))?;
        }
    }
}
