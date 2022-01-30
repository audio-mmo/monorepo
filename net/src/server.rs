use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tokio::net::TcpListener;
use tokio::sync::{mpsc::UnboundedReceiver, Semaphore};

use crate::connection::*;
use crate::network_connection::NetworkConnection;

#[derive(Clone, Debug, derive_builder::Builder)]
pub struct ServerConfig {
    pub interface: std::net::SocketAddr,
    pub port: usize,

    /// Maximum number of connections which may connect to the server at any one time.
    ///
    /// We start refusing connections if this many are open.
    #[builder(default = "512")]
    pub max_connections: usize,

    #[builder(default = "Default::default()")]
    pub connection_config: crate::network_connection::NetworkConnectionConfig,
}

pub struct Server {
    pub(crate) conn_sem: Arc<Semaphore>,
    pub(crate) config: ServerConfig,
    pub(crate) pending_connections_receiver:
        tokio::sync::Mutex<tokio::sync::mpsc::Receiver<Arc<dyn Connection>>>,
    pub(crate) pending_connections_sender: tokio::sync::mpsc::Sender<Arc<dyn Connection>>,
}

impl Server {
    pub fn new(config: ServerConfig) -> Arc<Server> {
        let (pending_connections_sender, pending_connections_receiver) =
            tokio::sync::mpsc::channel(config.max_connections);
        Arc::new(Server {
            conn_sem: Arc::new(Semaphore::new(config.max_connections)),
            config,
            pending_connections_sender,
            pending_connections_receiver: tokio::sync::Mutex::new(pending_connections_receiver),
        })
    }

    /// Drive the server's listening loop ona  Tokio runtime.
    pub async fn listening_loop(self: Arc<Self>) -> Result<()> {
        let listener = TcpListener::bind(self.config.interface).await?;
        loop {
            let permit = self.conn_sem.clone().acquire_owned().await?;
            let (stream, _) = listener.accept().await?;
            let conn = NetworkConnection::new(self.config.connection_config.clone());
            let sender = self.pending_connections_sender.clone();
            tokio::spawn(async {
                if let Err(e) = conn.task(stream, Some(permit), sender).await {
                    log::warn!("Error handling connection: {:?}", e);
                }
            });
        }
    }

    pub async fn await_connection(&self) -> Result<Option<Arc<dyn Connection>>> {
        Ok(self
            .pending_connections_receiver
            .lock()
            .await
            .recv()
            .await
            .map(Some)
            .unwrap_or(None))
    }
}
