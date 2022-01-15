use crate::header;

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum MessageKind {
    NotSimulation,
    Command,
    Event,
    Component,
    VisibilitySet,
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct MessageIdentifier {
    pub namespace: u8,
    pub id: u16,
}

pub struct Message<'a> {
    pub identifier: MessageIdentifier,
    pub kind: MessageKind,
    pub data: &'a [u8],
}

impl From<MessageKind> for header::HeaderKind {
    fn from(what: MessageKind) -> Self {
        match what {
            MessageKind::NotSimulation => header::HeaderKind::NotSimulation,
            MessageKind::Command => header::HeaderKind::Command,
            MessageKind::Event => header::HeaderKind::Event,
            MessageKind::Component => header::HeaderKind::Component,
            MessageKind::VisibilitySet => header::HeaderKind::VisibilitySet,
        }
    }
}

impl From<header::HeaderKind> for MessageKind {
    fn from(input: header::HeaderKind) -> MessageKind {
        match input {
            header::HeaderKind::NotSimulation => MessageKind::NotSimulation,
            header::HeaderKind::Command => MessageKind::Command,
            header::HeaderKind::Event => MessageKind::Event,
            header::HeaderKind::Component => MessageKind::Component,
            header::HeaderKind::VisibilitySet => MessageKind::VisibilitySet,
        }
    }
}

impl<'a> Message<'a> {
    pub(crate) fn len(&self) -> u64 {
        self.data.len() as u64
    }
}
