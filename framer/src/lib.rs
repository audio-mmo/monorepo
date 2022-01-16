//! This crate frames messages and parses framed messages.  To use, encode with a [Framer] and decode with a [Parser].
//!
//! This is a stateless network serializer/deserializer.  We support a variety of different message types, though we
//! don't put any particular meaning on the bytes in the message.  See [message::MessageKind] for the supported types
//! and their documentation.
//!
//! Each message also comes with a namespace and id pairing, which we use as a target to dispatch to a handler, e.g. the
//! chat system or a specific component.  The handlers are responsible for actually parsing the payloads out.  The
//! infrastructure for doing this is in the ammo_net crate.
mod framer;
mod header;
mod message;
mod parser;
#[cfg(test)]
mod tests;
mod varint;
pub use framer::*;
pub use message::*;
pub use parser::*;
