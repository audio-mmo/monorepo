use ammo_framer::Message;

use anyhow::Result;

pub trait Connection {
    fn receive_messages(&self, callback: &mut dyn FnMut(&Message) -> Result<()>) -> Result<()>;

    fn send_message(&self, message: &Message) -> Result<()>;

    fn is_connected(&self) -> bool;
}
