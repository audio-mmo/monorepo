use std::borrow::Cow;

use crate::header;

/// Kinds of message we support.
#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum MessageKind {
    /// This message is headed to something outside the simulation, for example a chat subsystem.
    NotSimulation,

    /// This message is a command, which is what the client sends to the server for actions.
    Command,

    /// This message is an event, which is what the server sends to the client for things like one-off sounds and such.
    Event,

    /// This message is a batch of components, which will be applied to the simulation.
    Components,

    /// This message specifies the visibility set, which is a list of all objects a given client can see.
    VisibilitySet,

    /// A client tick has ended.
    ///
    /// We distinguish the tick kinds because this allows increased genericity, and the ability to e.g. tell if client
    /// messages are being sent by the server.
    ClientTick,

    /// A server tick has ended.
    ServerTick,
}

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct MessageIdentifier {
    pub namespace: u8,
    pub id: u16,
}

#[derive(Debug, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct Message<'a> {
    pub identifier: MessageIdentifier,
    pub kind: MessageKind,
    pub data: Cow<'a, [u8]>,
}

impl From<MessageKind> for header::HeaderKind {
    fn from(what: MessageKind) -> Self {
        match what {
            MessageKind::NotSimulation => header::HeaderKind::NotSimulation,
            MessageKind::Command => header::HeaderKind::Command,
            MessageKind::Event => header::HeaderKind::Event,
            MessageKind::Components => header::HeaderKind::Component,
            MessageKind::VisibilitySet => header::HeaderKind::VisibilitySet,
            MessageKind::ClientTick => header::HeaderKind::ClientTick,
            MessageKind::ServerTick => header::HeaderKind::ServerTick,
        }
    }
}

impl From<header::HeaderKind> for MessageKind {
    fn from(input: header::HeaderKind) -> MessageKind {
        match input {
            header::HeaderKind::NotSimulation => MessageKind::NotSimulation,
            header::HeaderKind::Command => MessageKind::Command,
            header::HeaderKind::Event => MessageKind::Event,
            header::HeaderKind::Component => MessageKind::Components,
            header::HeaderKind::VisibilitySet => MessageKind::VisibilitySet,
            header::HeaderKind::ClientTick => MessageKind::ClientTick,
            header::HeaderKind::ServerTick => MessageKind::ServerTick,
        }
    }
}

impl<'a> Message<'a> {
    pub fn new(kind: MessageKind, identifier: MessageIdentifier, data: Cow<[u8]>) -> Message {
        Message {
            kind,
            identifier,
            data,
        }
    }

    /// Extend the lifetime of this message to 'static by cloning the data.
    pub fn clone_static(&self) -> Message<'static> {
        Message {
            kind: self.kind,
            identifier: self.identifier,
            data: Cow::Owned(self.data.to_vec()),
        }
    }

    pub(crate) fn len(&self) -> u64 {
        self.data.len() as u64
    }
}
