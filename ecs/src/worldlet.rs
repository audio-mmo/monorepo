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
use ammo_ecs_core::Component;
use anyhow::Result;
use atomic_refcell::{AtomicRef, AtomicRefMut};

use crate::store::Store;
use crate::store_map::StoreMap;
use crate::system::System;
use crate::system_map::SystemMap;
use crate::version::Version;

pub struct Worldlet<StoreM: StoreMap, SysM: SystemMap> {
    stores: StoreM,
    systems: SysM,

    /// Because we need to be generic over several type parameters, we use a vec of callbacks to know which systems to
    /// run.
    #[allow(clippy::type_complexity)]
    system_runners: Vec<fn(&Worldlet<StoreM, SysM>) -> Result<()>>,
}

impl<StoreM: StoreMap, SysM: SystemMap> Worldlet<StoreM, SysM> {
    fn register_system<S: System>(&mut self) {
        self.system_runners
            .push(|w| w.get_system_mut::<S>().execute(w));
        self.systems.register_system::<S>();
    }

    fn register_component<C: Component>(&mut self) {
        self.stores.register_component::<C>();
    }

    pub fn get_store<T: Component>(&self) -> AtomicRef<Store<T, Version>> {
        self.stores.get_store()
    }

    pub fn get_store_mut<T: Component>(&self) -> AtomicRefMut<Store<T, Version>> {
        self.stores.get_store_mut()
    }

    pub fn get_system<S: System>(&self) -> AtomicRef<S> {
        self.systems.get_system()
    }

    pub fn get_system_mut<S: System>(&self) -> AtomicRefMut<S> {
        self.systems.get_system_mut()
    }

    pub fn run_systems(&self) -> Result<()> {
        for f in self.system_runners.iter() {
            (*f)(self)?;
        }

        Ok(())
    }
}

/// A factory which can produce worldlets repeatedly.
#[derive(Default)]
pub struct WorldletFactory<StoreM: StoreMap, SysM: SystemMap> {
    /// Things we will do to the new worldlet.
    ops: Vec<fn(&mut Worldlet<StoreM, SysM>)>,

    pd: std::marker::PhantomData<(*mut StoreM, *mut SysM)>,
}

impl<StoreM: StoreMap, SysM: SystemMap> WorldletFactory<StoreM, SysM> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn register_system<S: System>(&mut self) -> &mut Self {
        self.ops.push(|w| {
            w.register_system::<S>();
        });
        self
    }

    pub fn register_component<C: Component>(&mut self) -> &mut Self {
        self.ops.push(|w| {
            w.register_component::<C>();
        });
        self
    }

    pub fn build_worldlet(&self) -> Worldlet<StoreM, SysM> {
        let mut worldlet = Worldlet {
            stores: Default::default(),
            systems: Default::default(),
            system_runners: Default::default(),
        };
        for o in self.ops.iter() {
            (*o)(&mut worldlet);
        }

        worldlet
    }
}
