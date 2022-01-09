use crate::header;
use crate::message::Message;
use crate::varint;

/// A framer writes frames to an internal buffer, then hands them out on request.
///
/// To use, call [Framer::encode_message] repeatedly, then [Framer::get_data], then [Framer::clear].  The general
/// pattern here is to build up the list of frames to send in a batch, then to read the data out and send them over the
/// network before repeating.
pub struct Framer {
    cap_limit: usize,
    buffer: Vec<u8>,
}

impl Framer {
    /// Create a framer.
    ///
    /// `cap_limit` is the maximum capacity of the internal buffer after clearing.  Used to make sure that large frame
    /// encodings don't cause
    pub fn new(cap_limit: usize) -> Framer {
        Framer {
            cap_limit,
            buffer: Vec::with_capacity(cap_limit),
        }
    }

    /// Clear the internal buffer to write a new batch of frames.
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.buffer.shrink_to(self.cap_limit);
    }

    pub fn add_message(&mut self, message: &Message) {
        use bytes::BufMut;

        let header = header::Header {
            kind: message.kind.into(),
            namespace: message.identifier.namespace,
            id: message.identifier.id,
        };

        varint::encode_varint(message.len(), &mut self.buffer);
        header.encode(&mut self.buffer);
        self.buffer.put(&mut &*message.data);
    }

    /// Read the data of all frames in the framer.
    pub fn get_data(&self) -> &[u8] {
        &self.buffer[..]
    }
}
