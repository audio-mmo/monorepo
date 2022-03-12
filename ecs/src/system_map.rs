use std::any::{Any, TypeId};
use std::collections::HashMap;

use atomic_refcell::{AtomicRef, AtomicRefCell, AtomicRefMut};

use crate::system::System;

/// The SystemMap trait represents maps of systems.
pub trait SystemMap: Sync + Send + 'static + Default {
    /// get a system.
    ///
    /// Should panic if the system is borrowed mutably, or doesn't exist.
    fn get_system<S: System>(&self) -> AtomicRef<S>;

    /// Get a system mutably.
    ///
    /// Panics if the system is borrowed immutably or doesn't exist.
    fn get_system_mut<S: System>(&self) -> AtomicRefMut<S>;

    /// Register a system with the map. Should be called once per system.
    fn register_system<S: System>(&mut self);
}

/// This is the most flexible but slow system map option, primarily useful for testing.
///
/// Real, prod-ready ECS setups will probably use fixed_typemap, but it is useful to have something quick to throw into
/// your unit tests#[derive(Default)]
#[derive(Default)]
pub struct DynamicSystemMap(HashMap<TypeId, Box<dyn Any + Send + Sync + 'static>>);

impl SystemMap for DynamicSystemMap {
    fn get_system<S: System>(&self) -> AtomicRef<S> {
        self.0
            .get(&TypeId::of::<AtomicRefCell<S>>())
            .expect("Should exist")
            .downcast_ref::<AtomicRefCell<S>>()
            .expect("Should downcast")
            .borrow()
    }

    fn get_system_mut<S: System>(&self) -> AtomicRefMut<S> {
        self.0
            .get(&TypeId::of::<AtomicRefCell<S>>())
            .expect("Should exist")
            .downcast_ref::<AtomicRefCell<S>>()
            .expect("Should downcast")
            .borrow_mut()
    }

    fn register_system<S: System>(&mut self) {
        let sys: AtomicRefCell<S> = Default::default();
        self.0
            .insert(TypeId::of::<AtomicRefCell<S>>(), Box::new(sys));
    }
}
