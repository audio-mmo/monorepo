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
pub trait Component:
    Serialize + DeserializeOwned + Clone + 'static + Send + Sync + crate::HasIdentifiers
{
}
