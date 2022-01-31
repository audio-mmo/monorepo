use ammo_framer::Message;

use anyhow::Result;

pub trait Connection: Send + Sync + 'static {
    /// The closure returns true to keep receiving messages, or false to stop.
    fn receive_messages(&self, callback: &mut dyn FnMut(&Message) -> Result<bool>) -> Result<()>;

    fn send_message(&self, message: &Message) -> Result<()>;

    fn is_connected(&self) -> bool;
}
