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
//!
//! Worldlets are build from a [WorldletTemplate], which is where systems, etc. are actually registered.
use crate::store_map::{StoreMap, StoreMapFactory, StoreMapFactoryBuilder, StoreRef, StoreRefMut};
use ammo_ecs_core::Component;

pub struct Worldlet {
    stores: StoreMap,
}

impl Worldlet {
    pub fn borrow_store<T: Component>(&self) -> StoreRef<T> {
        self.stores.borrow()
    }

    pub fn borrow_store_mut<T: Component>(&self) -> StoreRefMut<T> {
        self.stores.borrow_mut()
    }
}

pub struct WorldletTemplate {
    store_factory: StoreMapFactory,
}

impl WorldletTemplate {
    pub fn instantiate(&self) -> Worldlet {
        Worldlet {
            stores: self.store_factory.generate(),
        }
    }
}

#[derive(Default)]
pub struct WorldletTemplateBuilder {
    store_factory_builder: StoreMapFactoryBuilder,
}

impl WorldletTemplateBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn register_component<T: Component>(&mut self) -> &mut Self {
        self.store_factory_builder.register::<T>();
        self
    }

    pub fn build(self) -> WorldletTemplate {
        WorldletTemplate {
            store_factory: self.store_factory_builder.build(),
        }
    }
}
