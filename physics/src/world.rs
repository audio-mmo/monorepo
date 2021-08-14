//! The `World` is the main entrypoint to the library.
use std::cell::RefCell;
use std::rc::Rc;

use anyhow::Result;

use ammo_nzslab::{Slab, SlabHandle};

use crate::*;

pub(crate) struct WorldInner {
    pub(crate) bodies: RefCell<Slab<Body>>,
    /// Internal tag for the world, used to allow comparing `BodyHandle`.
    world_tag: usize,
}

fn get_world_tag() -> usize {
    use std::sync::atomic::*;

    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

impl WorldInner {
    pub fn new() -> WorldInner {
        WorldInner {
            bodies: Default::default(),
            world_tag: get_world_tag(),
        }
    }

    pub(crate) fn get_world_tag(&self) -> usize {
        self.world_tag
    }

    pub(crate) fn get_body(&self, slab_handle: &SlabHandle<Body>) -> std::cell::Ref<Body> {
        std::cell::Ref::map(self.bodies.borrow(), |x| x.get(slab_handle))
    }

    pub(crate) fn get_body_mut(&self, slab_handle: &SlabHandle<Body>) -> std::cell::RefMut<Body> {
        std::cell::RefMut::map(self.bodies.borrow_mut(), |x| x.get_mut(slab_handle))
    }

    pub(crate) fn remove_body(&self, slab_handle: SlabHandle<Body>) {
        self.bodies.borrow_mut().remove(slab_handle)
    }

    fn insert_body(&self, body: Body) -> SlabHandle<Body> {
        let mut slab = self.bodies.borrow_mut();
        slab.insert(body)
    }
}

impl Default for WorldInner {
    fn default() -> WorldInner {
        WorldInner::new()
    }
}

// A world is a collection of bodies that represent an environment.
pub struct World {
    inner: Rc<WorldInner>,
}

impl World {
    pub fn new() -> World {
        let inner = Rc::new(Default::default());
        World { inner }
    }

    fn new_handle_from_body(&self, body: Body) -> BodyHandle {
        let h = self.inner.insert_body(body);
        BodyHandle::new(&self.inner, h)
    }

    pub fn new_aabb(&self, center: V2, width: f64, height: f64) -> Result<BodyHandle> {
        let body = Body::new_aabb(center, width, height)?;
        Ok(self.new_handle_from_body(body))
    }

    pub fn new_circle(&self, center: V2, radius: f64) -> Result<BodyHandle> {
        let body = Body::new_circle(center, radius)?;
        Ok(self.new_handle_from_body(body))
    }
}

impl Default for World {
    fn default() -> World {
        World::new()
    }
}
