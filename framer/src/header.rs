/// A header for a message.  Contains the length, kind, namespace, and id.
///
/// On the wire,  this is parsed as a kind byte, a namespace id as a u8, and an id as a u16.
use bytes::{Buf, BufMut};

/// Size of the header, excluding length.
pub(crate) const HEADER_SIZE: u64 = 4;

#[derive(Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Debug)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub(crate) struct Header {
    pub(crate) kind: HeaderKind,
    pub(crate) namespace: u8,
    pub(crate) id: u16,
}

#[derive(Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Debug)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub(crate) enum HeaderKind {
    NotSimulation,
    Command,
    Event,
    Component,
    VisibilitySet,
}

#[derive(Debug, derive_more::Display, thiserror::Error)]
#[non_exhaustive]
pub enum HeaderDecodingError {
    NotEnoughData,
    InvalidHeaderKind(u8),
}

/// Used to convert header kinds to/from ints.  Must contain all header kinds.  This makes sure that we never
/// accidentally mismatch the to_int and from_int implementatison below.
static HEADER_LOOKUP_TABLE: [HeaderKind; 4] = [
    HeaderKind::NotSimulation,
    HeaderKind::Command,
    HeaderKind::Event,
    HeaderKind::Component,
];

impl HeaderKind {
    fn as_int(&self) -> u8 {
        for (i, v) in HEADER_LOOKUP_TABLE.iter().enumerate() {
            if *v == *self {
                return i as u8;
            }
        }

        panic!("Header kind not found in lookup table.");
    }

    fn from_int(val: u8) -> Result<HeaderKind, HeaderDecodingError> {
        HEADER_LOOKUP_TABLE
            .get(val as usize)
            .copied()
            .ok_or(HeaderDecodingError::InvalidHeaderKind(val))
    }
}

impl Header {
    pub(crate) fn encode(&self, dest: &mut impl BufMut) {
        dest.put_u8(self.kind.as_int());
        dest.put_u8(self.namespace);
        dest.put_u16(self.id);
    }

    pub(crate) fn decode(source: &mut impl Buf) -> Result<Header, HeaderDecodingError> {
        if (source.remaining() as u64) < HEADER_SIZE {
            return Err(HeaderDecodingError::NotEnoughData);
        }

        let kind = HeaderKind::from_int(source.get_u8())?;
        let namespace = source.get_u8();
        let id = source.get_u16();

        Ok(Header {
            id,
            kind,
            namespace,
        })
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
