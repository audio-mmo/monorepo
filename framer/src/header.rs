/// A header for a message.  Contains the length, kind, namespace, and id.
///
/// On the wire,  this is parsed as a kind byte, a namespace id as a u8, and an id as a u16.
use bytes::{Buf, BufMut};

/// Size of the header, excluding length.
pub(crate) const HEADER_SIZE: u64 = 3;

#[derive(Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Debug)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub(crate) struct Header {
    pub(crate) namespace: u8,
    pub(crate) id: u16,
}

#[derive(Debug, derive_more::Display, thiserror::Error)]
#[non_exhaustive]
pub enum HeaderDecodingError {
    NotEnoughData,
    InvalidHeaderKind(u8),
}

impl Header {
    pub(crate) fn encode(&self, dest: &mut impl BufMut) {
        dest.put_u8(self.namespace);
        dest.put_u16(self.id);
    }

    pub(crate) fn decode(source: &mut impl Buf) -> Result<Header, HeaderDecodingError> {
        if (source.remaining() as u64) < HEADER_SIZE {
            return Err(HeaderDecodingError::NotEnoughData);
        }

        let namespace = source.get_u8();
        let id = source.get_u16();

        Ok(Header { id, namespace })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use proptest::prelude::*;

    // The only real concern we have here is whether we hand-encode/hand-decode in the wrong order, so a simple
    // property-based test to fuzz it covers everything we care about.
    proptest! {
        #[test]
        fn test_fuzz_encoding(header: Header) {
            let mut buf = vec![];
            header.encode(&mut buf);
            let out = Header::decode(&mut &buf[..]).expect("Should decode");
            assert_eq!(header, out);
        }
    }
}
