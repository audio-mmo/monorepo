use std::borrow::Cow;

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct MessageIdentifier {
    pub namespace: u8,
    pub id: u16,
}

#[derive(Debug, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct Message<'a> {
    pub identifier: MessageIdentifier,
    pub data: Cow<'a, [u8]>,
}

impl<'a> Message<'a> {
    pub fn new(identifier: MessageIdentifier, data: Cow<[u8]>) -> Message {
        Message { identifier, data }
    }

    /// Extend the lifetime of this message to 'static by cloning the data.
    pub fn clone_static(&self) -> Message<'static> {
        Message {
            identifier: self.identifier,
            data: Cow::Owned(self.data.to_vec()),
        }
    }

    pub(crate) fn len(&self) -> u64 {
        self.data.len() as u64
    }
}
