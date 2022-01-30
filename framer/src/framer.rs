use crate::header;
use crate::message::Message;
use crate::varint;

/// The framer copies to the front of the buffer as bytes are consumed. We don't do this copy unless we will copy at
/// least this many bytes.
const MIN_ADVANCE_BY: usize = 8192;

/// Fraction at which to advance.
const ADVANCE_FRAC: f64 = 0.95;

pub struct Framer {
    buffer: Vec<u8>,
    cursor: usize,

    // We hold this in the struct so we can inject other values for testing.
    min_advance_by: usize,
}

impl Framer {
    pub fn new() -> Framer {
        Framer {
            buffer: vec![],
            cursor: 0,
            min_advance_by: MIN_ADVANCE_BY,
        }
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

    pub fn read_front(&self, amount: usize) -> &[u8] {
        let start = self.cursor;
        let end = (self.cursor + amount).min(self.buffer.len());
        &self.buffer[start..end]
    }

    pub fn advance_cursor(&mut self, by: usize) {
        self.cursor += by;
        assert!(self.cursor <= self.buffer.len());

        if self.cursor == self.buffer.len() {
            self.cursor = 0;
            self.buffer.clear();
            return;
        }

        if self.cursor < self.min_advance_by {
            return;
        }

        // If the buffer is growing forever, then we will never catch up and it doesn't matter how big it is.  There's a
        // few ways to know if data is being read faster than it is written, but one simple one is to advance the buffer
        // every time the reader passes a fractiaonl point.
        //
        // There is a degenerate case here where the buffer is (say) 32MB and growing by 1 byte a second, so in order to
        // counter that case we use a very high fraction.
        //
        // If this ever becomes a problem, we can switch to a proper growable ringbuffer.
        let cursor_frac = self.cursor as f64 / self.buffer.len() as f64;
        if cursor_frac < ADVANCE_FRAC {
            return;
        }

        let remaining = self.buffer.len() - self.cursor;
        self.buffer.copy_within(self.cursor.., 0);
        self.buffer.resize(remaining, 0);
        self.cursor = 0;
    }

    pub fn pending_bytes(&self) -> usize {
        self.buffer.len() - self.cursor
    }

    /// Steal this framer's data into a new framer, then reset this framer to be empty.
    ///
    /// This is useful when shutting down connections in the ammo_net crate, though there is likely to be a better
    /// design.
    pub fn steal(&mut self) -> Framer {
        let mut buffer = vec![];
        std::mem::swap(&mut self.buffer, &mut buffer);
        let ret = Framer {
            buffer,
            cursor: self.cursor,
            min_advance_by: self.min_advance_by,
        };
        *self = Framer::new();
        ret
    }
}

impl Default for Framer {
    fn default() -> Self {
        Framer::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor() {
        let mut framer = Framer::new();
        framer.buffer = (0..100).collect();
        framer.min_advance_by = 4;

        assert_eq!(framer.read_front(4), &[0, 1, 2, 3]);
        framer.advance_cursor(4);
        assert_eq!(framer.read_front(4), &[4, 5, 6, 7]);
        framer.advance_cursor(4);
        assert_eq!(framer.read_front(4), &[8, 9, 10, 11]);
        framer.advance_cursor(2);

        // We advanced by 10; we have 100 in the buffer. Let's go to 90.
        framer.advance_cursor(80);
        assert_eq!(framer.cursor, 90);

        // Now let's read the end.
        assert_eq!(
            framer.read_front(10),
            &[90, 91, 92, 93, 94, 95, 96, 97, 98, 99]
        );

        // This triggers an advancement.
        framer.advance_cursor(6);
        assert_eq!(framer.read_front(4), &[96, 97, 98, 99]);
        assert_eq!(framer.buffer.len(), 4);
    }
}
