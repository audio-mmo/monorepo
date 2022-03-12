use std::any::{Any, TypeId};
use std::collections::HashMap;

use atomic_refcell::{AtomicRef, AtomicRefCell, AtomicRefMut};

use ammo_ecs_core::Component;

use crate::store::Store;
use crate::version::Version;

/// The StoreMap trait represents maps of stores.
///
/// In order to allow for concurrency, we use AtomicRef and AtomicRefMut.
pub trait StoreMap: Sync + Send + 'static + Default {
    /// get a store, or insert an empty one.
    ///
    /// Should panic if the store is borrowed mutably.
    fn get_store<C: Component>(&self) -> AtomicRef<Store<C, Version>>;

    /// Get a store mutably, or insert an empty one.
    ///
    /// Panics if the store is borrowed immutably.
    fn get_store_mut<C: Component>(&self) -> AtomicRefMut<Store<C, Version>>;

    /// register a component with this map.
    ///
    /// Makes sure the store is present.
    ///
    /// Will be called exactly once per component.
    fn register_component<C: Component>(&mut self);
}

/// This is the most flexible but slow store map option, primarily useful for testing.
///
/// Real, prod-ready ECS setups will probably use fixed_typemap, but it is useful to have something quick to throw into
/// your unit tests.
#[derive(Default)]
pub struct DynamicStoreMap(HashMap<TypeId, Box<dyn Any + Send + Sync + 'static>>);

impl StoreMap for DynamicStoreMap {
    fn get_store<C: Component>(&self) -> AtomicRef<Store<C>> {
        self.0
            .get(&TypeId::of::<AtomicRefCell<Store<C>>>())
            .expect("Should exist")
            .downcast_ref::<AtomicRefCell<Store<C>>>()
            .expect("Should downcast")
            .borrow()
    }

    fn get_store_mut<C: Component>(&self) -> AtomicRefMut<Store<C>> {
        self.0
            .get(&TypeId::of::<AtomicRefCell<Store<C>>>())
            .expect("Should exist")
            .downcast_ref::<AtomicRefCell<Store<C>>>()
            .expect("Should downcast")
            .borrow_mut()
    }

    fn register_component<C: Component>(&mut self) {
        let store: AtomicRefCell<Store<C>> = Default::default();
        self.0
            .insert(TypeId::of::<AtomicRefCell<Store<C>>>(), Box::new(store));
    }
}
