use std::any::{Any, TypeId};
use std::collections::HashMap;

use atomic_refcell::AtomicRefCell;

use crate::component::Component;

use crate::store::Store;
use crate::version::Version;

/// The StoreMap trait represents maps of stores.
///
/// In order to allow for concurrency, we use AtomicRef and AtomicRefMut.
pub trait StoreMap: Sync + Send + 'static + Default {
    /// get a store.
    ///
    /// Should panic if the store isn't present.
    fn get_store<C: Component>(&self) -> &AtomicRefCell<Store<C, Version>>;

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
    fn get_store<C: Component>(&self) -> &AtomicRefCell<Store<C>> {
        self.0
            .get(&TypeId::of::<AtomicRefCell<Store<C>>>())
            .expect("Should exist")
            .downcast_ref::<AtomicRefCell<Store<C>>>()
            .expect("Store should be registered first")
    }

    fn register_component<C: Component>(&mut self) {
        let store: AtomicRefCell<Store<C>> = Default::default();
        self.0
            .insert(TypeId::of::<AtomicRefCell<Store<C>>>(), Box::new(store));
    }
}
