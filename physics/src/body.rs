use std::rc::{Rc, Weak};

use anyhow::{Context, Result};
use arrayvec::ArrayVec;

use ammo_nzslab::SlabHandle;

use crate::*;

/// The number of proposed movements allowed per iteration.
const MAX_PROPOSED_MOVEMENTS: usize = 3;

#[derive(Clone, Debug)]
pub struct ProposedMovement {
    /// The absolute position to move the body to.
    pub(crate) new_position: V2,
    /// A usize value which can be used to link this movement to user-defined data.
    pub(crate) user_id: usize,
}

#[derive(Clone, Debug)]
pub struct MovementResult {
    pub new_position: V2,
    pub user_id: usize,
}

impl ProposedMovement {
    pub fn new_with_id(new_position: &V2, id: usize) -> ProposedMovement {
        ProposedMovement {
            new_position: *new_position,
            user_id: id,
        }
    }

    pub fn new(new_position: &V2) -> ProposedMovement {
        ProposedMovement::new_with_id(new_position, 0)
    }

    /// Get the id associated with this `ProposedMovement`.
    pub fn get_user_id(&self) -> usize {
        self.user_id
    }
}

/// A body represents an entity in the world.
///
/// The external type to interact with bodies is actually the [BodyHandle], which refers to a body.
#[derive(Debug)]
pub(crate) struct Body {
    pub(crate) position: V2,
    pub(crate) shape: Shape,
    pub(crate) proposed_movements: ArrayVec<ProposedMovement, MAX_PROPOSED_MOVEMENTS>,
    pub(crate) movement_results: ArrayVec<MovementResult, MAX_PROPOSED_MOVEMENTS>,
    /// Starts at 0. Incremented when handles are created.
    pub(crate) refcount: usize,
}

impl Body {
    pub(crate) fn new_aabb(center: V2, width: f64, height: f64) -> Result<Body> {
        let half_dims = V2::new(width / 2.0, height / 2.0);
        let aabb = Aabb::from_points(center - half_dims, center + half_dims)?;
        Ok(Body {
            position: center,
            shape: aabb.into(),
            proposed_movements: Default::default(),
            movement_results: Default::default(),
            // Not a mistake: starts at 0 until wrapped in a handle.
            refcount: 0,
        })
    }

    pub(crate) fn new_circle(center: V2, radius: f64) -> Result<Body> {
        let c = Circle::new(center, radius)?;
        Ok(Body {
            position: center,
            shape: c.into(),
            proposed_movements: Default::default(),
            movement_results: Default::default(),
            // Not a mistake: start at 0 until wrapped in a handle.
            refcount: 0,
        })
    }

    /// Move the body.  This is a direct movement and doesn't account for collisions.
    pub fn move_body(&mut self, new_center: &V2) {
        self.position = *new_center;
        self.shape = self.shape.move_shape(new_center);
    }

    /// Propose a movement, which may or may not be applied the next time this body runs through the physics/collision tick.
    pub fn propose_movement(&mut self, new_center: &V2) -> Result<()> {
        let proposed = ProposedMovement::new(new_center);
        self.proposed_movements
            .try_push(proposed)
            .context("Too many proposed movements")
    }
}

/// A reference to a  body.  This is a reference-counted handle which compares
/// equal with handles that refer to the same body.
///
/// the handle keeps an internal weak reference to a world.  If the world dies
/// before the handle, all setters start doing nothing and all getters start
/// erroring.
pub struct BodyHandle {
    slab_handle: SlabHandle<Body>,
    world: Weak<WorldInner>,
    world_tag: usize,
}

impl BodyHandle {
    /// Create a `BodyHandle`, incrementing the body's refcount.
    pub(crate) fn new(world: &Rc<WorldInner>, slab_handle: SlabHandle<Body>) -> BodyHandle {
        world.get_body_mut(&slab_handle).refcount += 1;
        BodyHandle {
            slab_handle,
            world_tag: world.get_world_tag(),
            world: Rc::downgrade(world),
        }
    }

    /// Directly move this body immediately.
    ///
    /// Doesn't check collision. Just immediately moves the body.
    pub fn move_body(&mut self, new_center: &V2) {
        if let Some(world) = self.world.upgrade() {
            world.get_body_mut(&self.slab_handle).move_body(new_center)
        }
    }

    /// Propose a movement for a body.
    pub fn propose_movement(&mut self, new_center: &V2) -> Result<()> {
        if let Some(world) = self.world.upgrade() {
            world
                .get_body_mut(&self.slab_handle)
                .propose_movement(new_center)?;
        }

        Ok(())
    }
}

impl Drop for BodyHandle {
    fn drop(&mut self) {
        let world = match self.world.upgrade() {
            Some(x) => x,
            None => return,
        };

        let will_delete = {
            let mut body = world.get_body_mut(&self.slab_handle);
            body.refcount -= 1;
            body.refcount == 0
        };
        if will_delete {
            // We can't move, the borrow checker isn't smart enough.
            world.remove_body(self.slab_handle.clone());
        }
    }
}

impl std::cmp::PartialEq for BodyHandle {
    fn eq(&self, other: &Self) -> bool {
        let t1 = (self.slab_handle.get_tag(), self.world_tag);
        let t2 = (other.slab_handle.get_tag(), other.world_tag);
        t1 == t2
    }
}

impl Eq for BodyHandle {}

impl std::cmp::PartialOrd for BodyHandle {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let t1 = (self.slab_handle.get_tag(), self.world_tag);
        let t2 = (other.slab_handle.get_tag(), other.world_tag);
        t1.partial_cmp(&t2)
    }
}

impl std::cmp::Ord for BodyHandle {
    fn cmp(&self, other: &BodyHandle) -> std::cmp::Ordering {
        let t1 = (self.slab_handle.get_tag(), self.world_tag);
        let t2 = (other.slab_handle.get_tag(), other.world_tag);
        t1.cmp(&t2)
    }
}

impl std::hash::Hash for BodyHandle {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        state.write_usize(self.slab_handle.get_tag());
        state.write_usize(self.world_tag);
    }
}

impl Clone for BodyHandle {
    fn clone(&self) -> BodyHandle {
        if let Some(world) = self.world.upgrade() {
            let mut body = world.get_body_mut(&self.slab_handle);
            body.refcount += 1;
        };

        BodyHandle {
            slab_handle: self.slab_handle.clone(),
            world: self.world.clone(),
            world_tag: self.world_tag,
        }
    }
}
