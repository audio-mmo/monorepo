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

/// Both components and systems need identifiers.  This trait and the accompanying derive allow for this logic to be
/// shared.
pub trait HasIdentifiers {
    fn get_int_id(&self) -> IntId {
        Self::get_int_id_from_type()
    }
    fn get_string_id(&self) -> StringId {
        Self::get_string_id_from_type()
    }

    fn get_int_id_from_type() -> IntId;
    fn get_string_id_from_type() -> StringId;
}
