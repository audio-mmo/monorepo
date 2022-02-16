use std::num::NonZeroU16;

use derive_more::Display;

/// A string namespace/id pair, used to identify things for e.g. database tables.
#[derive(Copy, Clone, Debug, Display, Ord, Eq, PartialOrd, PartialEq, Hash)]
#[display(fmt = "{}/{}", namespace, id)]
pub struct StringId {
    pub namespace: &'static str,
    pub id: &'static str,
}

/// An int-based namespace/id pair, used to identify things for e.g. the network.
#[derive(Copy, Clone, Debug, Display, Ord, Eq, PartialOrd, PartialEq, Hash)]
#[display(fmt = "(namespace={}, id={})", namespace, id)]
pub struct IntId {
    pub namespace: NonZeroU16,
    pub id: u16,
}
