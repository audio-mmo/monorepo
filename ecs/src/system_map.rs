//! This is like the [crate::store_map::StoreMap] but for systems.
//!
//! Unfortunately Rust doesn't give us a good way to deduplicate this code; if they remain sufficiently similar, we may
//! opt to do so in future via macros.
//!
//! We don't bother to hide that we're using [AtomicRefCell] because this is only ever used in one place: though the
//! structs are public, we don't expect people to use it outside our infrastructure.
use std::any::TypeId;

use atomic_refcell::{AtomicRefCell, AtomicRefMut};

use crate::frozen_map::{FrozenMap, FrozenMapBuilder};
use crate::system::System;

pub struct SystemMap {
    inner: FrozenMap<TypeId, AtomicRefCell<Box<dyn System>>>,
}

impl SystemMap {
    /// Iterate over all systems, producing a mutable borrow.
    pub fn iter_mut(&self) -> impl Iterator<Item = AtomicRefMut<Box<dyn System>>> {
        self.inner.values().map(|x| x.borrow_mut())
    }
}

type SystemMapInserter = Box<dyn Fn(&mut FrozenMapBuilder<TypeId, AtomicRefCell<Box<dyn System>>>)>;

pub struct SystemMapFactory(Vec<SystemMapInserter>);

impl SystemMapFactory {
    pub fn generate(&self) -> SystemMap {
        let mut mb = FrozenMapBuilder::new();
        for i in self.0.iter() {
            (*i)(&mut mb);
        }
        SystemMap { inner: mb.freeze() }
    }
}

#[derive(Default)]
pub struct SystemMapFactoryBuilder(Vec<SystemMapInserter>);

impl SystemMapFactoryBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add_system<T: System + Default>(&mut self) -> &mut Self {
        self.add_system_with_builder::<T, _>(Default::default)
    }

    pub fn add_system_with_builder<T: System, B: Fn() -> T + 'static>(
        &mut self,
        builder: B,
    ) -> &mut Self {
        self.0.push(Box::new(move |m| {
            m.add(TypeId::of::<T>(), AtomicRefCell::new(Box::new(builder())));
        }));
        self
    }

    pub fn build(self) -> SystemMapFactory {
        SystemMapFactory(self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::worldlet::Worldlet;

    use anyhow::Result;

    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    // These set their flag when they're run.
    struct DummySystem1(Arc<AtomicBool>);
    struct DummySystem2(Arc<AtomicBool>);

    impl System for DummySystem1 {
        fn execute(&mut self, _: &Worldlet) -> Result<()> {
            self.0.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    impl System for DummySystem2 {
        fn execute(&mut self, _: &Worldlet) -> Result<()> {
            self.0.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    #[test]
    fn test_basic() {
        let flag1 = Arc::new(AtomicBool::new(false));
        let f1_cloned = flag1.clone();
        let flag2 = Arc::new(AtomicBool::new(false));
        let f2_cloned = flag2.clone();

        let mut fb = SystemMapFactoryBuilder::new();
        fb.add_system_with_builder::<DummySystem1, _>(move || DummySystem1(f1_cloned.clone()));
        fb.add_system_with_builder::<DummySystem2, _>(move || DummySystem2(f2_cloned.clone()));
        let factory = fb.build();

        let worldlet = crate::worldlet::WorldletTemplateBuilder::new()
            .build()
            .instantiate();

        for _ in 0..2 {
            let map = factory.generate();
            for mut i in map.iter_mut() {
                let _ = i.execute(&worldlet);
            }

            assert!(flag1.load(Ordering::Relaxed));
            assert!(flag2.load(Ordering::Relaxed));
            flag1.store(false, Ordering::SeqCst);
            flag2.store(false, Ordering::SeqCst);
        }
    }
}
