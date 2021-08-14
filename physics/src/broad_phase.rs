//! The broad phase iterates over all bodies in the world, builds a spatial hash, and then resolves collisions.
use ammo_nzslab::*;

use crate::*;

/// How much dilation to apply to the boxes? Prevents floating point rounding
/// from being a problem.
const DILATION_FACTOR: f64 = 1.01;

pub(crate) fn execute_broad_phase(world: &mut WorldInner, cell_width: u32, cell_height: u32) {
    let mut body_slab = world.bodies.borrow_mut();
    let mut ref_slab: Slab<&mut Body> = Slab::new();
    let mut hash: SpatialHash<SlabHandle<&mut Body>> = SpatialHash::new(cell_width, cell_height);

    for body in body_slab.iter_mut() {
        let aabb = body.get_bounding_box().dilate(DILATION_FACTOR);
        hash.insert(&aabb, ref_slab.insert(body));
    }

    for i in hash.iter_all_possible_collisions() {
        // Possible place to optimize allocation; Rust doesn't like pulling the
        // vector out of the loop because of the mutable borrows.

        let mut materialized_bodies = vec![];
        materialized_bodies.clear();
        unsafe { materialized_bodies.extend(ref_slab.slab_handles_to_mut_refs(i)) };
        collide_slice(&mut materialized_bodies[..]);
    }
}
