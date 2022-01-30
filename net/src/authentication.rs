use std::sync::Arc;

use anyhow::Result;

use ammo_framer::Message;

use crate::Connection;

pub trait Authenticator: Send + Sync + 'static {
    fn authenticate(&self, first_msg: &Message, conn: Arc<dyn Connection>) -> Result<()>;
}
