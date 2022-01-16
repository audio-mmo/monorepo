//! The Component trait.
//!
//! The Component trait defines a component, under the standard ECS definition.
//! Uniquely to us, components must also define a unique string namespace/pair
//! and integer namespace/pair for serialization, networking, and other internal
//! purposes.  See the documentation on the trait for more.
use std::num::NonZeroU16;

use derive_more::*;
use serde::{de::DeserializeOwned, Serialize};

#[derive(Copy, Clone, Debug, Display, Ord, Eq, PartialOrd, PartialEq, Hash)]
#[display(fmt = "{}/{}", namespace, id)]
pub struct StringComponentId {
    pub namespace: &'static str,
    pub id: &'static str,
}

#[derive(Copy, Clone, Debug, Display, Ord, Eq, PartialOrd, PartialEq, Hash)]
#[display(fmt = "(namespace={}, id={})", namespace, id)]
pub struct IntComponentId {
    pub namespace: NonZeroU16,
    pub id: u16,
}

/// A component, for example an object position.
///
/// Components must implement [Serialize] and [DeserializeOwned] as well as the methods on this trait.
pub trait Component: Serialize + DeserializeOwned + Clone + 'static {
    /// Get the string-based namespace for this component, for example `("ammo",
    /// "position")`.  The string namespace `"ammo"` is reserved.
    ///
    /// If two components with the same namespace/name pairing are registered
    /// with the ECS, a panic results because this is a programmer error.
    fn get_string_id() -> StringComponentId
    where
        Self: Sized;

    /// Get the integer version of the components namespace/name tuple in the
    /// form `(namespace, component_id)`.
    ///
    /// Namespace must be nonzero.  The namespace `1` is reserved.
    ///
    /// It is a large aid to efficiency and memory usage if namespaces and ids
    /// are as small as possible. Prefer to use the smallest namespace possible,
    /// and to number components sequentially from 0.
    fn get_int_id() -> IntComponentId
    where
        Self: Sized;
}

/// A trait providing a blanket impl to add object-safe forms of the [Component] type-level metods, as well as other component helpers.
pub trait ComponentExt: Component {
    fn get_int_id(&self) -> IntComponentId;
    fn get_string_id(&self) -> StringComponentId;
}

impl<T: Component> ComponentExt for T {
    fn get_int_id(&self) -> IntComponentId {
        Self::get_int_id()
    }

    fn get_string_id(&self) -> StringComponentId {
        Self::get_string_id()
    }
}
