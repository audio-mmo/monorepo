use std::sync::Arc;

use anyhow::Result;

use ammo_framer::Message;

/// What to do after handling a message.
#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MessageHandlerOutcome {
    /// Continue reading messages from the stream.
    ContinueHandling,

    /// Gracefully shut down by flushing any enqueued messages.
    GracefulShutdown,

    /// Immediately throw out the connection.
    ImmediateShutdown,
}

/// This trait represents the ability to handle messages, and is implemented differently for both the client and server
/// half of a connection.
///
/// We use it to allow zero-copy decoding from messages to their expanded protobuf representations.  When a connection
/// is created, this is the incoming half of the stream.
pub trait MessageHandler: Send + Sync {
    fn handle_message(&self, message: &Message) -> Result<MessageHandlerOutcome>;
}

/// This is a factory which creates message handlers based off the first message received.  It is used only by the
/// server.  The first message is typically an authentication message.
///
/// The passed in channel may be used to send messages in the other direction, and is typically associated with an
/// in-game entity for that purpose.
pub trait MessageHandlerFactory: Send + Sync {
    type Handler: MessageHandler;

    fn handle_incoming_message(
        &self,
        message: &Message,
        sender: tokio::sync::mpsc::UnboundedSender<Message<'static>>,
    ) -> Result<Self::Handler>;
}

impl<T: MessageHandlerFactory> MessageHandlerFactory for Arc<T> {
    type Handler = T::Handler;

    fn handle_incoming_message(
        &self,
        message: &Message,
        sender: tokio::sync::mpsc::UnboundedSender<Message<'static>>,
    ) -> Result<Self::Handler> {
        (**self).handle_incoming_message(message, sender)
    }
}
