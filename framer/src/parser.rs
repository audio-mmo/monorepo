use std::borrow::Cow;

use bytes::Buf;

use crate::header;
use crate::message;
use crate::varint;

/// A parser parses frames.
///
/// To use, call [Parser::feed] with some data, then repeatedly call [Parser::read_message] and [Parser::roll_forward]
/// in a loop until read_message asks for more data to get messages out.
///
/// It is safe to feed this parser with more data than is in a single message; the next messages will be picked up next
/// time.  Note, however, that this is optimized for small reads: as messages are extracted we copy data to the front of
/// the buffer.
pub struct Parser {
    length_limit: Option<u64>,
    cap_limit: usize,
    buffer: Vec<u8>,
}

pub enum ParserOutcome<'a> {
    Message(message::Message<'a>),
    /// More data is required.  Returns a lower bound on how much data this actually is.
    MoreDataRequired(u64),
}

#[derive(Debug, derive_more::Display, thiserror::Error)]
pub enum ParserError {
    MessageTooLong,

    /// Happens if a message claims to contain less data than the header size.
    MessageTooShort,

    HeaderDecodingError(#[from] header::HeaderDecodingError),

    VarintError(#[from] varint::VarintError),
}

impl Parser {
    /// Create a parser.
    ///
    /// `length_limit` is an optional limit on the length of messages.  `cap_limit` is a limit on the size of the
    /// internal buffer when it's empty (but individual messages can be longer).
    pub fn new(length_limit: Option<u64>, cap_limit: usize) -> Parser {
        Parser {
            length_limit,
            cap_limit,
            buffer: Vec::with_capacity(cap_limit),
        }
    }

    /// Feed the parser with some bytes.
    pub fn feed(&mut self, bytes: &mut impl Buf) -> Result<(), ParserError> {
        use bytes::BufMut;
        self.buffer.put(bytes);

        if let Some(l) = self.length_limit {
            if !self.buffer.is_empty() {
                let length_so_far = match varint::decode_varint(&mut &self.buffer[..]) {
                    Ok(i) => i,
                    Err(varint::VarintError::Incomplete(i)) => i,
                    Err(e) => return Err(e.into()),
                };

                if length_so_far > l {
                    return Err(ParserError::MessageTooLong);
                }
            }
        }

        Ok(())
    }

    /// try to read a message, if possible.
    pub fn read_message(&mut self) -> Result<ParserOutcome, ParserError> {
        if self.buffer.is_empty() {
            return Ok(ParserOutcome::MoreDataRequired(0));
        }

        let mut buf = &self.buffer[..];
        let varint_res = varint::decode_varint(&mut buf);
        if let Err(varint::VarintError::Incomplete(val)) = varint_res {
            // If the varint is longer than the length limit, we can bail now.
            if let Some(l) = self.length_limit {
                if val < l as u64 {
                    return Err(ParserError::MessageTooLong);
                }
            }

            // We don't know how long varints actually are, so we can't return anything but 0.
            return Ok(ParserOutcome::MoreDataRequired(0));
        }
        let length = varint_res?;

        if length < header::HEADER_SIZE {
            return Err(ParserError::MessageTooShort);
        }

        if let Some(l) = self.length_limit {
            if length > l {
                return Err(ParserError::MessageTooLong);
            }
        }

        if header::HEADER_SIZE > length {
            return Err(ParserError::MessageTooShort);
        }

        // We yanked the varint off the internal buffer already.
        if (buf.len() as u64) < length {
            return Ok(ParserOutcome::MoreDataRequired(length - buf.len() as u64));
        }

        // Ok, we have enough data. Get the header:
        let header = header::Header::decode(&mut buf)?;
        let message = message::Message {
            identifier: message::MessageIdentifier {
                namespace: header.namespace,
                id: header.id,
            },
            kind: header.kind.into(),
            data: Cow::Borrowed(&buf[..(length - header::HEADER_SIZE) as usize]),
        };

        Ok(ParserOutcome::Message(message))
    }

    /// Roll forward past the first message in this parser, if possible.  Should only be called after read_message
    /// returns a message.
    pub fn roll_forward(&mut self) -> Result<(), ParserError> {
        let mut buf = &self.buffer[..];
        let len = varint::decode_varint(&mut buf)?;
        // Length of the varint.
        let var_len = self.buffer.len() - buf.len();
        let total_len = len as usize + var_len;
        let buf_len = self.buffer.len();
        self.buffer.copy_within(total_len..buf_len, 0);
        self.buffer.resize(self.buffer.len() - total_len, 0);
        self.buffer.shrink_to(self.cap_limit);
        Ok(())
    }

    /// Get the number of bytes in this parser.
    ///
    /// Decreases when [Parser::roll_forward] is called.
    pub fn contained_bytes(&self) -> usize {
        self.buffer.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_length_limit() {
        let mut parser = Parser::new(Some(10), 1024);

        // Our message: 11 bytes, header is NotSimulation id (0, 0),Rest of the data is just zeroed.
        let data = vec![11, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let err = parser.feed(&mut &data[..]).err().expect("should be error");
        assert!(matches!(err, ParserError::MessageTooLong));
    }
}
