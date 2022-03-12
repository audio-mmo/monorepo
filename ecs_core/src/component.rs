//! The Component trait.
//!
//! The Component trait defines a component, under the standard ECS definition.
//! Uniquely to us, components must also define a unique string namespace/pair
//! and integer namespace/pair for serialization, networking, and other internal
//! purposes.  See the documentation on the trait for more.
use serde::{de::DeserializeOwned, Serialize};

/// A component, for example an object position.
///
/// Components must implement [Serialize] and [DeserializeOwned] as well as the methods on this trait.
pub trait Component: Serialize + DeserializeOwned + Clone + 'static + Send + Sync {
    /// Get the string-based namespace for this component, for example `("ammo",
    /// "position")`.  The string namespace `"ammo"` is reserved.
    ///
    /// If two components with the same namespace/name pairing are registered
    /// with the ECS, a panic results because this is a programmer error.
    fn get_string_id() -> crate::StringId
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
    fn get_int_id() -> crate::IntId
    where
        Self: Sized;
}

/// A trait providing a blanket impl to add object-safe forms of the [Component] type-level metods, as well as other
/// component helpers.
pub trait ComponentExt: Component {
    fn get_int_id(&self) -> crate::IntId;
    fn get_string_id(&self) -> crate::StringId;
}

impl<T: Component> ComponentExt for T {
    fn get_int_id(&self) -> crate::IntId {
        Self::get_int_id()
    }

    fn get_string_id(&self) -> crate::StringId {
        Self::get_string_id()
    }
}
