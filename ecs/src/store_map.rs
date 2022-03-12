

use atomic_refcell::{AtomicRef, AtomicRefMut};

use ammo_ecs_core::Component;

use crate::store::Store;
use crate::version::Version;

/// The StoreMap trait represents maps of stores.
///
/// In order to allow for concurrency, we use AtomicRef and AtomicRefMut.
pub trait StoreMap: Sync + Send + 'static {
    /// get a store, or insert an empty one.
    ///
    /// Should panic if the store is borrowed mutably.
    fn get_store<'a, C: Component>(&'a self) -> AtomicRef<'a, Store<C, Version>>;

    /// Get a store mutably, or insert an empty one.
    ///
    /// Panics if the store is borrowed immutably.
    fn get_store_mut<'a, C: Component>(&'a self) -> AtomicRefMut<'a, Store<C, Version>>;
}
