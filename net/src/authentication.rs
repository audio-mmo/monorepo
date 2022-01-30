use std::sync::Arc;

use anyhow::Result;

use ammo_framer::Message;

use crate::Connection;

/// Authenticate a connection, wiring it up to whatever it needs to be wired up to.
pub trait Authenticator: Send + Sync + 'static {
    fn authenticate(&self, first_msg: &Message, conn: Arc<dyn Connection>) -> Result<()>;
}
