//! The [StoreMap] contains a collection of component stores, and is constructed via a [StoreMapFactory], which you get
//! via a [StoreMapFactoryBuilder].
//!
//! The reason this is so overengineered is that we want to be able to get multiple maps from a factory, and we want the
//! user to configure one factory for us containing all the types they want stores for.  Specifically:
//!
//! - At program start, the user registers all their types.  This becomes a factory.
//! - At zone load, the factory generates a map for the zone.
//!
//! The map offers a `RefCell`-like interface: calling `borrow` returns a wrapper of an immutable borrow, calling
//! `borrow_mut` returns a wrapper of a mutable borrow but panics if there is an immutable borrow, and calling `get_mut`
//! always succeeds because the caller must have started with a `&mut` reference.
//!
//! The map panics if it's asked for a type it doesn't contain: this is a coding error.
use std::any::{Any, TypeId};

use atomic_refcell::{AtomicRef, AtomicRefCell, AtomicRefMut};

use crate::frozen_map::{FrozenMap, FrozenMapBuilder};
use crate::store::Store;

/// A mutable borrow of a store.
pub struct StoreRef<'a, T>(AtomicRef<'a, Store<T>>);

/// A mutable borrow of a store.
pub struct StoreRefMut<'a, T>(AtomicRefMut<'a, Store<T>>);

impl<'a, T> std::ops::Deref for StoreRef<'a, T> {
    type Target = Store<T>;
    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<'a, T> std::ops::Deref for StoreRefMut<'a, T> {
    type Target = Store<T>;
    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<'a, T> std::ops::DerefMut for StoreRefMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.deref_mut()
    }
}

/// A map which contains a collection of stores.
pub struct StoreMap(FrozenMap<TypeId, Box<dyn Any>>);

impl StoreMap {
    fn get_refcell<T: 'static>(&self) -> &AtomicRefCell<Store<T>> {
        self.0
            .get(&TypeId::of::<T>())
            .expect("Should find the specified type in the map")
            .downcast_ref()
            .expect("Should always downcast")
    }

    fn get_refcell_mut<T: 'static>(&mut self) -> &mut AtomicRefCell<Store<T>> {
        self.0
            .get_mut(&TypeId::of::<Store<T>>())
            .expect("Should find the specified type in the map")
            .downcast_mut()
            .expect("Should always downcast")
    }

    /// Borrow a store immutably. Panics if there is an outstanding mutable borrow.
    pub fn borrow<T: 'static>(&self) -> StoreRef<T> {
        StoreRef(self.get_refcell().borrow())
    }

    /// Borrow a store mutably. Panics if there are any other borrows.
    pub fn borrow_mut<T: 'static>(&self) -> StoreRefMut<T> {
        StoreRefMut(self.get_refcell().borrow_mut())
    }

    /// Get a mutable reference to a store. Panics if the store is not in the map.
    ///
    /// This always succeeds if the type is present, since having a mutable reference as the caller is a proof that no
    /// immutable borrows are outstanding.
    pub fn get_mut<T: 'static>(&mut self) -> &mut Store<T> {
        self.get_refcell_mut().get_mut()
    }
}

/// Callback type used below in the factory.
///
/// This erases generics by instantiating a function implementation which will add a map entry to a `FrozenMapBuilder`,
/// then storing those in a vec.
type StoreMapInserter = fn(&mut FrozenMapBuilder<TypeId, Box<dyn Any>>) -> ();

/// helper method, placed in a vec of callbacks to represent what types to add to the map at runtime.
fn insert_store<T: Any>(builder: &mut FrozenMapBuilder<TypeId, Box<dyn Any>>) {
    builder.add(
        TypeId::of::<T>(),
        Box::new(AtomicRefCell::new(Store::<T>::new())),
    );
}

pub struct StoreMapFactory(Vec<StoreMapInserter>);

impl StoreMapFactory {
    pub fn generate(&self) -> StoreMap {
        let mut mb = FrozenMapBuilder::new();
        for i in self.0.iter() {
            (*i)(&mut mb);
        }
        StoreMap(mb.freeze())
    }
}

#[derive(Default)]
pub struct StoreMapFactoryBuilder(Vec<StoreMapInserter>);

impl StoreMapFactoryBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    /// Registera type with this map.
    pub fn register<T: Any>(&mut self) -> &mut Self {
        self.0.push(insert_store::<T>);
        self
    }

    pub fn build(self) -> StoreMapFactory {
        StoreMapFactory(self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Type1(u64);
    struct Type2(u64);

    fn build_test_map() -> StoreMap {
        let mut fb = StoreMapFactoryBuilder::new();
        fb.register::<Type1>().register::<Type2>();
        let fact = fb.build();
        fact.generate()
    }

    #[test]
    fn test_basic() {
        let map = build_test_map();
        {
            let _s1 = map.borrow::<Type1>();
            let _s2 = map.borrow::<Type2>();
        }
        // And now a mutable borrow shouldn't panic.
        map.borrow::<Type1>();
    }

    #[test]
    #[should_panic]
    fn test_immutable_mutable_fails() {
        let map = build_test_map();
        let _borrow = map.borrow::<Type1>();
        map.borrow_mut::<Type1>();
    }
}
