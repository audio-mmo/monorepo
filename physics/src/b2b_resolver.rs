//! This module implements the functionality necessary to resolve the collision
//! between a pair of bodies.
//!
//! This physics library implements an energy-reducing algorithm which prefers
//! simplicity over realism.  For every movement of two bodies:
//!
//! - If they're already colliding, always allow movement.  This can make
//!   tunnelling worse, but will let the simulation trend toward not having
//!   collisions.
//! - If moving both bodies causes a collision, deny and/or reduce the movement
//!   of both bodies.
//! - Then, check the pairs, and if keeping one body stationary and moving the
//!   other causes a collision, deny/reduce the movement of the moving body
use crate::*;

fn movement_ok(s1: &Shape, np1: &V2, s2: &Shape, np2: &V2) -> bool {
    let f1 = s1.move_shape(np1);
    let f2 = s2.move_shape(np2);
    !f1.collides_with(&f2)
}

/// Check two bodies against each other, using the first proposed movement for each.
///
/// Returns a tuple `(b1, b2)` specifying which body is allowed to move.
fn check_body_pair_one(b1: &Body, np1: &V2, b2: &Body, np2: &V2) -> (bool, bool) {
    if b1.shape.collides_with(&b2.shape) || movement_ok(&b1.shape, np1, &b2.shape, np2) {
        (true, true)
    } else if movement_ok(&b1.shape, np1, &b2.shape, &V2::new(0.0, 0.0)) {
        (true, false)
    } else if movement_ok(&b1.shape, &V2::new(0.0, 0.0), &b2.shape, np2) {
        (false, true)
    } else {
        (false, false)
    }
}

/// Check a pair of bodies against each other, continually popping off proposed
/// movements from either body depending on which one can't move.
///
/// Return if any movements were disallowed.
fn check_body_pair(b1: &Body, b2: &Body) -> (usize, usize) {
    let i1: usize = 0;
    let i2: usize = 0;
    let mut d1: usize = 1;
    let mut d2: usize = 1;

    // Check i1 and i2, then if one of the movements is denied set that body's
    // d1 and/or d2 to 0 so that it's index stops increasing.  The indices
    // will then be one past the last allowed movement for that body.
    while d1 != 0 || d2 != 0 {
        let np1 = b1.proposed_movements.get(0).map(|x| x.new_position);
        let np2 = b2.proposed_movements.get(i2).map(|x| x.new_position);

        let (np1, np2) = match (np1, np2) {
            (Some(x), Some(y)) => (x, y),
            (Some(x), None) => (x, V2::new(0.0, 0.0)),
            (None, Some(x)) => (V2::new(0.0, 0.0), x),
            (None, None) => {
                break;
            }
        };

        let (nd1, nd2) = check_body_pair_one(b1, &np1, b2, &np2);
        d1 = nd1 as usize;
        d2 = nd2 as usize;
    }

    (i1, i2)
}

/// Check a slice of bodies.
///
/// Iterates over the slice until there are no movements left.  Effectively
/// `O(n^2)` for the time being.  This is a good optimization point.  In
/// particular, applying sort and sweep on the aabbs of the bodies would be an
/// easy and effective way to reduce the cost.  This implementation is primarily
/// for prototyping purposes; we can improve it once we're sure that we like
/// what's going on.
///
/// Used as the backend after the spatial hash is built.  The slices should be
/// all bodies which can possibly interact.
fn collide_slice(bodies: &mut [&mut Body]) {
    let mut keep_going = true;
    let slice_len = bodies.len();

    while keep_going {
        for i in 0..slice_len {
            for j in 0..i {
                let (i1, i2) = check_body_pair(bodies[i], bodies[j]);
                let old_b1_len = bodies[i].proposed_movements.len();
                let old_b2_len = bodies[i].proposed_movements.len();
                bodies[i].proposed_movements.truncate(i1);
                bodies[j].proposed_movements.truncate(i2);
                keep_going = old_b1_len != i1 || old_b2_len != i2;
            }
        }
    }

    // Now figure out the results.
    bodies.iter_mut().for_each(|b| {
        b.movement_results.clear();
        (*b).movement_results = b
            .proposed_movements
            .iter()
            .map(|x| MovementResult {
                new_position: x.new_position,
                user_id: x.user_id,
            })
            .collect();
        (*b).position = b
            .proposed_movements
            .last()
            .map(|x| x.new_position)
            .unwrap_or_else(|| b.position);
        (*b).proposed_movements.clear();
    });
}
