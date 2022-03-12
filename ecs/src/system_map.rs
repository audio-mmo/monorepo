use atomic_refcell::{AtomicRef, AtomicRefMut};

use crate::system::System;

/// The SystemMap trait represents maps of systems.
pub trait SystemMap: Sync + Send + 'static + Default {
    /// get a system, or insert an empty one.
    ///
    /// Should panic if the system is borrowed mutably.
    fn get_system<S: System>(&self) -> AtomicRef<S>;

    /// Get a system mutably, or insert an empty one.
    ///
    /// Panics if the system is borrowed immutably.
    fn get_system_mut<S: System>(&self) -> AtomicRefMut<S>;
}
