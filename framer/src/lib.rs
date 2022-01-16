//! This crate frames messages and parses framed messages.  To use, encode with a [Framer] and decode with a [Parser].
//!
//! This is a stateless network serializer/deserializer.  We support 4 message types:
//!
//! - NotSimulation: things like login, logout, chat, etc.  Stuff that doesn't fall cleanly into one category and isn't
//!   part of syncing the object model.
//! - Component: a request to sync a component.
//! - Command: a game-related command, e.g. "the player pressed a button".  usually client->server.
//! - Event: "this happened", e.g. "damage" or whatever, usually server->client.
//!
//! This crate doesn't understand what the messages mean, just how to separate them into kinds.  You get a slice of u8
//! out, and are responsible for decoding it, e.g. from protobuf.
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
