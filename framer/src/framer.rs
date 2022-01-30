use crate::header;
use crate::message::Message;
use crate::varint;

/// A framer writes frames to an internal buffer, then hands them out on request.
///
/// To use, call [Framer::encode_message] repeatedly, then [Framer::get_data], then [Framer::clear].  The general
/// pattern here is to build up the list of frames to send in a batch, then to read the data out and send them over the
/// network.  The batch interface allows for ammo_net to perform larger writes.
pub struct Framer {
    buffer: Vec<u8>,
}

impl Framer {
    pub fn new() -> Framer {
        Framer { buffer: vec![] }
    }

    /// Clear the internal buffer to write a new batch of frames.
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    pub fn add_message(&mut self, message: &Message) {
        use bytes::BufMut;

        let header = header::Header {
            kind: message.kind.into(),
            namespace: message.identifier.namespace,
            id: message.identifier.id,
        };

        varint::encode_varint(message.len() + header::HEADER_SIZE, &mut self.buffer);
        header.encode(&mut self.buffer);
        self.buffer.put(&mut &*message.data);
    }

    /// Read the data of all frames in the framer.
    pub fn get_data(&self) -> &[u8] {
        &self.buffer[..]
    }
}

impl Default for Framer {
    fn default() -> Self {
        Framer::new()
    }
}
