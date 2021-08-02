use crate::raycasting::*;
use crate::*;

/// A ray-aabb intersection test using the slab test, with a modification that allows it to also compute the normal.
pub(crate) fn ray_aabb_test(ray: &Ray, aabb: &Aabb) -> Option<RaycastingResult> {
    // The one fun thing about this algorithm is that we have to be careful
    // about NaN.  Rust's built-in min/max return the non-NaN number and thus
    // always produce a number unless both arguments are NaN.  The comments in
    // this function point out when this may occur, and how to deal with it.

    // First, we compute the inverse of the ray's directions for efficiency,
    // because division is expensive.  Note that if the array aligns with one of
    // the axis, one of these inverses is infinity.
    let inv_dx = 1.0 / ray.direction.x;
    let inv_dy = 1.0 / ray.direction.y;

    // For convenience, extract the extents of the box.
    let bmin_x = aabb.get_p1().x;
    let bmax_x = aabb.get_p2().x;
    let bmin_y = aabb.get_p1().y;
    let bmax_y = aabb.get_p2().y;

    // In 2d, planes are actually lines.  The box can be visualized as having 2
    // x lines and 2 y lines, coming out of the left/right sides of the box and
    // top/bottom respectively.  A ray only intersects the box if it intersects
    // both pairs of lines at the same time.
    //
    // To understand why, consider that asking if a point is inside the box is
    // the same as asking if the points' min/max x are in range *and* the points
    // min/max y are in range.  If the ray intersects both pairs of lines at the
    // same time, we can use this to prove that there is a point inside the box.
    //
    // We want to be branchless as much as possible.  We can't avoid branches at
    // the end, but we can avoid all but 1 branch in the case of a missing ray.
    // To do so, unconditionally calculate all 4 possible t values instead of
    // baling early.
    //
    // Finally, it is not at this point clear which t values are less: if the
    // ray comes from the left then `xt1 < xt2` otherwise maybe not.  We deal
    // with that case later.
    //
    // This is one of the aforementioned points at which this function can have
    // NaN, since if the ray is aligned with an axis and on the edge of the box,
    // we end up with `inf * 0.0`.  To deal with this case, we say that in that
    // case the ray doesn't intersect the box, and let the NaN infect; when we
    // get to checking whether or not there can be an intersection, all the
    // conditions start returning false and it's fine.
    //
    // The subtractions here translate the box's min/max points into the coordinate space of the ray.
    let tx1 = (bmin_x - ray.origin.x) * inv_dx;
    let tx2 = (bmax_x - ray.origin.x) * inv_dx;
    let ty1 = (bmin_y - ray.origin.y) * inv_dy;
    let ty2 = (bmax_y - ray.origin.y) * inv_dy;

    // Now, we need to figure out if the intervals overlap.  It would seem that this is as simple as flipping tx1 and tx2 etc. around to figure out which is minimal, but that is unfortunately not the case since those values can be NaN.
    //
    // First, observe that either both tx1 and tx2 are NaN or both ty1 and ty2 are NaN.  This means that even though Rust suppresses NaN, we can get NaN for the initial tmin and tmax:
    let mut tmin = tx1.min(tx2);
    let mut tmax = tx1.max(tx2);

    // But now it's tricky: either tmin and tmax aren't NaN, or the values we're
    // about to produce aren't NaN.  If we then simply apply it, we either keep
    // the previous values (a non-empty range) or replace the previous values
    // (also a non-empty range).  What we really want is a min/max which returns
    // NaN.
    //
    // Unfortunately such an implementation involves branches, but we are also
    // happy if `min(a, naN) = min(naN, b) = INF` and `max(a, NaN) = max(NaN, b)
    // = -INF`, which will produce an empty range.
    //
    // Finally, if all values aren't NaN, what we want to do is increase tmin to
    // the new tmin of the y range, and decrease tmax to the max of the y range.
    // We don't let them move in the other direction: this constraint means that
    // either tmin moves to be greater than tmax of the x range, or tmax moves
    // to be less than tmin of the x range when they don't overlap.
    tmin = tmin.max(ty1.min(ty2).min(f64::INFINITY));
    tmax = tmax.min(ty1.max(ty2).max(f64::NEG_INFINITY));

    // There are no NaN after this point in the function.

    // The t on the ray is tmin, but constrained to be positive.  We keep actual
    // tmin around for the inside check.
    let tray = tmin.max(0.0);

    // We don't intersect if `tray >= tmax`.  If `tray > tmax`, the ranges
    // didn't overlap at all.  Otherwise the ray is on the edge of the box, and
    // we drop that case to be consistent with the NaN version of this function.
    if tray >= tmax {
        return None;
    }

    // The point on the ray where the intersection is.
    let point = ray.evaluate(tray);

    if tmin < 0.0 {
        return Some(RaycastingResult {
            inside: true,
            normal: None,
            point,
        });
    }

    // Now for the normal.  This works as follows.  We only used min and max on
    // our t values and we know the ray was outside the box.  This means that
    // tray must equal one of the 4 values.  We also know which sides they go
    // with.
    let normal = if tray == tx1.min(tx2) {
        V2::new(-1.0, 0.0)
    } else if tray == tx1.max(tx2) {
        V2::new(1.0, 0.0)
    } else if tray == ty1.min(ty2) {
        V2::new(0.0, -1.0)
    } else if tray == ty1.max(ty2) {
        V2::new(0.0, 1.0)
    } else {
        unreachable!("The ray must equal one of the 4 t values");
    };

    Some(RaycastingResult {
        point,
        inside: false,
        normal: Some(normal),
    })
}
