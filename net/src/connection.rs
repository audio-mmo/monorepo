use ammo_framer::Message;

use anyhow::Result;

/// Trait representing the ability to send and receive messages.
///
/// when dropped, the connection closes.
pub trait Connection {
    /// Read some messages from the connection, if any.
    ///
    /// Calls the callback on the messages, which opens us up to the ability to do zero-copy deserialization.
    fn receive_messages(&self, callback: &dyn FnMut(&Message) -> Result<()>) -> Result<()>;

    /// Send a message.
    fn send_message(&self, message: &Message) -> Result<()>;
}
