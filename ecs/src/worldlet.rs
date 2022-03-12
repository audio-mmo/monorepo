//! A worldlet is the basic unit of execution.
//!
//! Worldlets combine various maps with interior mutability into a combined structure, which can be operated on by
//! systems and commands.  On the server, a worldlet will have server-side systems and commands; on the client, by
//! contrast, it will have rimarily presentation functionality registered.  The key distinguishing factor between
//! clients and servers with respect to the simulationitself is what is registered and whether or not more than one
//! worldlet is ever active at a time.
//!
//! Worldlets are always run on exactly one thread at a time, but not necessarily the same thread.  Though it is in
//! theory possible for systems, etc. to "escape" their zone, they should not do so, and the API is designed to make
//! this somewhat difficult.  If a system is able to use parallelism specifically within this worldlet, it can do so via
//! Rayon.
use atomic_refcell::{AtomicRef, AtomicRefMut};

use ammo_ecs_core::Component;

use crate::store::Store;
use crate::store_map::StoreMap;
use crate::version::Version;

pub struct Worldlet<SM: StoreMap> {
    stores: SM,
}

impl<SM: StoreMap> Worldlet<SM> {
    pub fn get_store<T: Component>(&self) -> AtomicRef<Store<T, Version>> {
        self.stores.get_store()
    }

    pub fn get_store_mut<T: Component>(&self) -> AtomicRefMut<Store<T, Version>> {
        self.stores.get_store_mut()
    }
}
