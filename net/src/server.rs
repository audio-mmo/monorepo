use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tokio::net::TcpListener;
use tokio::sync::{mpsc::UnboundedReceiver, Semaphore};

use crate::authentication::*;
use crate::network_connection::NetworkConnection;

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
    connection_config: crate::network_connection::NetworkConnectionConfig,
}

pub struct Server {
    pub(crate) conn_sem: Arc<Semaphore>,
    pub(crate) config: ServerConfig,
    pub(crate) authenticator: Arc<dyn Authenticator>,
}

impl Server {
    pub fn new<Auth: Authenticator>(config: ServerConfig, authenticator: Auth) -> Arc<Server> {
        Arc::new(Server {
            conn_sem: Arc::new(Semaphore::new(config.max_connections)),
            authenticator: Arc::new(authenticator),
            config,
        })
    }

    /// Drive the server's listening loop ona  Tokio runtime.
    pub async fn listening_loop(self: Arc<Self>) -> Result<()> {
        let listener = TcpListener::bind(self.config.interface).await?;
        loop {
            let permit = self.conn_sem.clone().acquire_owned().await?;
            let (stream, _) = listener.accept().await?;
            let conn = NetworkConnection::new(
                self.config.connection_config.clone(),
                Some(self.authenticator.clone()),
            );
            tokio::spawn(async {
                if let Err(e) = conn.task(stream, Some(permit)).await {
                    log::warn!("Error handling connection: {:?}", e);
                }
            });
        }
    }
}
