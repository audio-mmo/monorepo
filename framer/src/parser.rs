use std::borrow::Cow;

use bytes::Buf;

use crate::header;
use crate::message;
use crate::varint;

/// A parser parses frames.
///
/// To use, call [Parser::feed] with some data, which will return a [ParserOutcome] (either a message or request for
/// more data) or error out if it will no longer be possible to decode (because, e.g., we had a message longer than a
/// length limit or invalid data).  Once you get and process a message, call [Parser::roll_forward], which will drop the
/// message from the head of the parser's buffer, so that the next message may be returned.
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
    pub fn feed(&mut self, bytes: &mut impl Buf) -> Result<ParserOutcome, ParserError> {
        use bytes::BufMut;

        self.buffer.put(bytes);
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
            data: Cow::Borrowed(buf),
        };

        Ok(ParserOutcome::Message(message))
    }

    /// Drop the first message off the front of the parser.
    ///
    /// Errors if called while the parser contains an invalid message.
    pub fn roll_forward(&mut self) -> Result<(), ParserError> {
        let vlen;
        let length;

        {
            let mut buf = &self.buffer[..];
            length = varint::decode_varint(&mut buf)?;
            // use the fact that decoding the varint moved the buffer forward to determine how long the varint was.
            vlen = self.buffer.len() - buf.len();
        }

        let old_len = self.buffer.len();
        (&mut self.buffer[..]).copy_within((vlen + length as usize)..old_len, 0);
        self.buffer
            .resize(self.buffer.len() - length as usize - vlen, 0);
        self.buffer.shrink_to(self.cap_limit);
        Ok(())
    }
}
