use crate::raycasting::*;

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

    // Now, we need to figure out if the intervals overlap.  It would seem that
    // this is as simple as flipping tx1 and tx2 etc. around to figure out which
    // is minimal, but that is unfortunately not the case since those values can
    // be NaN.
    //
    // First, observe that either both tx1 and tx2 are NaN or both ty1 and ty2
    // are NaN.  This means that even though Rust suppresses NaN, we can get NaN
    // for the initial tmin and tmax.  We go ahead and convert these to `tmin =
    // inf` and `tmax = -inf`.
    let txmin = tx1.min(tx2).min(f64::INFINITY);
    let txmax = tx1.max(tx2).max(f64::NEG_INFINITY);

    // Apply the same logic for a tymin and tymax.
    let tymin = ty1.min(ty2).min(f64::INFINITY);
    let tymax = ty1.max(ty2).max(f64::NEG_INFINITY);

    // tmin is the maximum of the lower points of the range because if the ranges overlap, the highest low point has to fit.
    let tmin = txmin.max(tymin);
    // And tmax is the min of the maxes, by similar logic.
    let tmax = txmax.min(tymax);

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
    let normal = if tray == tx1 {
        V2::new(-1.0, 0.0)
    } else if tray == tx2 {
        V2::new(1.0, 0.0)
    } else if tray == ty1 {
        V2::new(0.0, -1.0)
    } else if tray == ty2 {
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

#[cfg(test)]
mod tests {
    use super::*;

    use proptest::prelude::*;

    /// cast a ray against a given AABB, checking that it hits, is outside the
    /// box, and has the required normal.
    fn check_normal(
        ray: &Ray,
        aabb: &Aabb,
        expected_normal: &V2,
    ) -> prop::test_runner::TestCaseResult {
        let test_res = ray_aabb_test(ray, aabb).unwrap();
        prop_assert!(!test_res.inside);
        let norm = test_res
            .normal
            .expect("This test should always produce normals");
        prop_assert_eq!(&norm, expected_normal, "{:?} {:?}", ray, aabb);
        let point_dist = aabb.distance_to_point(&test_res.point);
        prop_assert!(point_dist < 0.001);
        Ok(())
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100000))]
        #[test]
        fn hit_fuzz(
            // Positions of the center and size of the box.
            orig_x in -1000.0..1000.0f64,
            orig_y in -1000.0..1000.0f64,
            box_width in 1.0..100.0f64,
            box_height in 1.0..100.0f64,
            // Used to determine the target point in the box.
            point_x_percent in 0.01..0.99f64,
            point_y_percent in 0.01..0.99f64,
            // Used to determine the source point. Added to the center of the box.
            source_point_x in -1000.0..1000.0f64,
            source_point_y in -1000.0..1000.0f64,
        ) {
            let minx = orig_x - box_width / 2.0;
            let miny  = orig_y - box_height / 2.0;
            let maxx = orig_x + box_width / 2.0;
            let maxy  = orig_y + box_height / 2.0;
            let aabb = Aabb::from_points(V2::new(minx, miny), V2::new(maxx, maxy)).expect("Should succeed");
            // Add in a bit so we don't get a ray of 0 length.
            let source_x = orig_x + source_point_x + 0.01;
            let source_y = orig_y + source_point_y + 0.01;

            let target_x = minx + box_width * point_x_percent;
            let target_y = miny + box_height * point_y_percent;

            let ray = Ray::from_points(V2::new(source_x, source_y), V2::new(target_x, target_y));

            let test_res = ray_aabb_test(&ray, &aabb);
            prop_assert!(test_res.is_some());
            let inner = test_res.unwrap();

            // If the source point is inside the box, we can do some additional checks.
            if minx < source_x && source_x < maxx &&
                miny < source_y && source_y < maxy {
                    prop_assert!(inner.inside);
                    // This is actually exactly equal because in this case everything is `* 0.0`.
                    prop_assert_eq!(inner.point, V2::new(source_x, source_y));
            }
        }

        // This test works by working out the radius of a sphere slightly larger than the box, then firing points pointing away from the box.
        #[test]
        fn miss_fuzz(
            orig_x in -1000.0..1000.0f64,
            orig_y in -1000.0..1000.0f64,
            box_width in 1.0..100.0f64,
            box_height in 1.0..100.0f64,
            angle in 0.0..100.0f64,
            radius_multiplier in 1.01..2.0f64,
            ray_length in 1.0..100.0f64,
        ) {
            let minx = orig_x - box_width / 2.0;
            let maxx = orig_x + box_width / 2.0;
            let miny = orig_y - box_height / 2.0;
            let maxy = orig_y + box_height / 2.0;

            // Work out the radius.
            let rad = V2::new(orig_x, orig_y).distance(&V2::new(minx, miny)) * radius_multiplier;
            let aabb = Aabb::from_points(V2::new(minx, miny), V2::new(maxx, maxy)).unwrap();
            let ray = Ray::new(V2::new(orig_x + angle.cos() * rad, orig_y + angle.sin() * rad),
                V2::new(angle.cos(), angle.sin()),
                ray_length);
            assert!(ray_aabb_test(&ray, &aabb).is_none());
        }

        #[test]
        fn test_normals(
            orig_x in -1000.0..1000.0f64,
            orig_y in -1000.0..1000.0f64,
            box_width in 1.0..100.0f64,
            box_height in 1.0..100.0f64,
            box_dist in 0.1..100.0f64,
            side_percent in 0.01..0.99f64,
            target_x_percent in 0.1..0.99f64,
            target_y_percent in 0.1..0.99f64,
        ) {
            let minx = orig_x - box_width / 2.0;
            let maxx = orig_x + box_width / 2.0;
            let miny = orig_y  - box_height / 2.0;
            let maxy = orig_y + box_height / 2.0;

            let aabb = Aabb::from_points(V2::new(minx, miny), V2::new(maxx, maxy)).expect("Should build");

            let target_x = minx + box_width * target_x_percent;
            let target_y = miny + box_height * target_y_percent;

            let x_per = minx + box_width * side_percent;
            let y_per = miny + box_height * side_percent;

            // Table is (source, normal).
            let cases: Vec<(V2, V2)> = vec![
                (V2::new(minx - box_dist, y_per), V2::new(-1.0, 0.0)),
                (V2::new(maxx + box_dist, y_per), V2::new(1.0, 0.0)),
                (V2::new(x_per, miny - box_dist), V2::new(0.0, -1.0)),
                (V2::new(x_per, maxy + box_dist), V2::new(0.0, 1.0)),
            ];

            for (source, normal) in cases.into_iter() {
                let ray = Ray::from_points(source, V2::new(target_x, target_y));
                check_normal(&ray, &aabb, &normal)?;
            }
        }
    }

    // This test sets up some test cases where we want to be sure that the ray
    // doesn't ever hit the box when aligned with a side.  Checks NaN handling.
    #[test]
    fn test_edges() {
        let aabb = Aabb::from_points(V2::new(-1.0, -1.0), V2::new(1.0, 1.0)).unwrap();
        // Bind the V2 constructors so we can easily loop over coordinates.
        let builders: Vec<fn(offset: f64) -> V2> = vec![
            |x| V2::new(x, -1.0),
            |x| V2::new(x, 1.0),
            |x| V2::new(-1.0, x),
            |x| V2::new(1.0, x),
        ];
        // The extents we want to check:
        let extents = vec![
            (-5.0, 5.0),
            (-1.0, 1.0),
            (0.0, 1.0),
            (0.0, 5.0),
            (-5.0, 0.0),
            (0.0, -10.0),
            (0.0, 10.0),
        ];

        for (start, end) in extents.into_iter() {
            for builder in builders.iter() {
                let ray = Ray::from_points(builder(start), builder(end));
                assert!(ray_aabb_test(&ray, &aabb).is_none(), "{:?}", ray);
            }
        }
    }
}
