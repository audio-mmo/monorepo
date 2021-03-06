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
use anyhow::Result;
use atomic_refcell::AtomicRefCell;

use crate::component::Component;
use crate::stores::Store;
use crate::stores::StoreMap;
use crate::system::System;
use crate::system_map::SystemMap;
use crate::version::Version;

/// Trait representing a system in an object-safe fashion.
///
/// We need to be able to do stuff to systems, but the system trait isn't actually object safe.  To get around this, we
/// must use this trait to build up vtables of the operations we wish to perform.  We then use references to zero-sized
/// statics to have references to the vtables.
trait SystemVtable<WorldletT> {
    fn execute(&self, worldlet: &WorldletT) -> Result<()>;
}

#[derive(Default)]
struct VtableInstance<S>(std::marker::PhantomData<*mut S>);

impl<S: System, StoreM: StoreMap, SystemM: SystemMap> SystemVtable<Worldlet<StoreM, SystemM>>
    for VtableInstance<S>
{
    fn execute(&self, worldlet: &Worldlet<StoreM, SystemM>) -> Result<()> {
        worldlet.get_system::<S>().borrow_mut().execute(worldlet)
    }
}

pub struct Worldlet<StoreM: StoreMap, SysM: SystemMap> {
    stores: StoreM,
    systems: SysM,

    /// Because we need to be generic over several type parameters, we use a vec of callbacks to know which systems to
    /// run.
    #[allow(clippy::type_complexity)]
    system_vtables: Vec<&'static dyn SystemVtable<Worldlet<StoreM, SysM>>>,
}

impl<StoreM: StoreMap, SysM: SystemMap> Worldlet<StoreM, SysM> {
    fn register_system<S: System>(&mut self) {
        self.system_vtables
            .push(&VtableInstance::<S>(std::marker::PhantomData));
        self.systems.register_system::<S>();
    }

    fn register_component<C: Component>(&mut self) {
        self.stores.register_component::<C>();
    }

    pub fn get_store<T: Component>(&self) -> &AtomicRefCell<Store<T, Version>> {
        self.stores.get_store()
    }

    pub fn get_system<S: System>(&self) -> &AtomicRefCell<S> {
        self.systems.get_system()
    }

    pub fn run_systems(&self) -> Result<()> {
        for f in self.system_vtables.iter() {
            f.execute(self)?;
        }

        Ok(())
    }
}

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
            system_vtables: Default::default(),
        };
        for o in self.ops.iter() {
            (*o)(&mut worldlet);
        }

        worldlet
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::stores::DynamicStoreMap;
    use crate::system_map::DynamicSystemMap;

    use crate::HasIdentifiers;

    #[derive(Default)]
    struct System1(bool);
    #[derive(Default)]
    struct System2(bool);

    impl System for System1 {
        fn execute<StoreM: StoreMap, SysM: SystemMap>(
            &mut self,
            _worldlet: &Worldlet<StoreM, SysM>,
        ) -> Result<()> {
            self.0 = true;
            Ok(())
        }
    }

    impl System for System2 {
        fn execute<StoreM: StoreMap, SysM: SystemMap>(
            &mut self,
            _worldlet: &Worldlet<StoreM, SysM>,
        ) -> Result<()> {
            self.0 = true;
            Ok(())
        }
    }

    #[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
    struct Comp {
        data: u32,
    }

    impl HasIdentifiers for Comp {
        fn get_string_id_from_type() -> crate::StringId {
            crate::StringId {
                namespace: "ammo",
                id: "comp",
            }
        }

        fn get_int_id_from_type() -> crate::IntId {
            crate::IntId {
                id: 1,
                namespace: std::num::NonZeroU16::new(1).unwrap(),
            }
        }
    }

    impl Component for Comp {}

    fn factory() -> WorldletFactory<DynamicStoreMap, DynamicSystemMap> {
        let mut fact = WorldletFactory::new();
        fact.register_system::<System1>()
            .register_system::<System2>()
            .register_component::<Comp>();
        fact
    }

    #[test]
    fn test_getting() {
        let fact = factory();
        let worldlet = fact.build_worldlet();

        // These panic if they're broken.
        worldlet.get_store::<Comp>();
        worldlet.get_system::<System1>();
        worldlet.get_system::<System2>();
        worldlet.get_store::<Comp>();
        worldlet.get_system::<System1>();
        worldlet.get_system::<System2>();
    }

    #[test]
    fn test_running() {
        let worldlet = factory().build_worldlet();
        worldlet.run_systems().expect("Should run");
        assert!(worldlet.get_system::<System1>().borrow().0);
        assert!(worldlet.get_system::<System2>().borrow().0);
    }
}
