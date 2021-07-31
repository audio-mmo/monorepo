//! Test a ray against a circle.
use crate::raycasting::*;
use crate::*;

pub(crate) fn ray_circle_test(ray: &Ray, circle: &Circle) -> Option<RaycastingResult> {
    // How this works is as follows: if we project the vector from the start of
    // the ray to the center of the circle onto the ray itself, we get a t value
    // which is the t for which the ray is closest to the circle.  If we project
    // the circle onto a ray which is rotated 90 degrees, we get the distance
    // from the circle to the ray.
    //
    // We can use the distance from the circle to the ray to know if we hit.
    // Then, using the fact that circles are symmetrical, we can use
    // `sqrt(r^2-x^2)` where x is the distance from the center of the circle to
    // the ray to determine how wide the line segment which intersects the
    // circle is, and then `t_intersect+-delta` gets us the two t values on the
    // ray which pass through the circle.

    // Translated center of the circle, so that the ray is at the origin.
    let translated_center = V2::new(
        circle.get_center().x - ray.origin.x,
        circle.get_center().y - ray.origin.y,
    );
    // Dot this with the rotated version of the ray's direction. Which way doesn't matter if weuse abs.
    let dist_proj = V2::new(ray.direction.y, -ray.direction.x)
        .dot(&translated_center)
        .abs();
    if dist_proj > circle.get_radius() {
        return None;
    }

    let t_centered = ray.direction.dot(&translated_center);
    let rad = (circle.get_radius().powi(2) - dist_proj.powi(2)).sqrt();
    let t1 = t_centered - rad;
    let t2 = t_centered + rad;

    // t2 > t1 because `rad >= 0`.  If `t2 < 0` then `t1 < 0` and the circle is
    // entirely behind the ray.
    //
    // Otherwise, if `t1 > ray_length`, the point isn't on the ray, and we also
    // didn't hit.
    //
    // The two remaining cases are either (1) `t1 < 0 < t2` in which case the
    // ray starts inside the circle, or (2) `t1 < ray_len` in which case the ray
    // starts outside the circle.
    //
    // For (1), we have no normal because the normal is ambiguous.  In the other
    // case, the normal is the normalized vector from the center of the sphere
    // to the point of intersection.

    // First, get rid of any ray that definitely doesn't hit.
    if t2 < 0.0 || t1 > ray.length {
        return None;
    }

    // The ray starts inside.
    if t1 <= 0.0 {
        return Some(RaycastingResult {
            point: ray.origin,
            normal: None,
            inside: true,
        });
    }

    // Otherwise we have a normal.  First work out the point of intersection.
    let point = V2::new(
        ray.origin.x + t1 * ray.direction.x,
        ray.origin.y + t1 * ray.direction.y,
    );
    // And the normal:
    let normal = V2::new(
        circle.get_center().x - point.x,
        circle.get_center().y - point.y,
    )
    .normalize();
    Some(RaycastingResult {
        point,
        normal: Some(normal),
        inside: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use proptest::prelude::*;

    #[test]
    fn test_simple() {
        let circ = Circle::new(V2::new(2.0, 0.5), 1.0).expect("Should work");
        let ray_hit = Ray::new(V2::new(1.0, 0.0), V2::new(1.0, 0.0), 5.0);
        let ray_miss = Ray::new(V2::new(1.0, 0.0), V2::new(-1.0, 0.0), 5.0);
        let hit_test = ray_circle_test(&ray_hit, &circ);
        let miss_test = ray_circle_test(&ray_miss, &circ);
        assert!(hit_test.is_some(), "{:?}", hit_test);
        assert!(miss_test.is_none(), "{:?}", miss_test);
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100000))]

        // Fire a bunch of rays at a sphere. They must intersect, and the point
        // of intersection must be on the sphere.  Additionally a ray fired in
        // the opposite direction must not intersect the sphere.
        #[test]
        fn test_fuzz(
            // Generate an origin.
            x in -100.0..100.0f64,
            y in -100.0..100.0f64,
            // Size and position of the circle.
            radius in 1.0..10.0f64,
            // We do circle positions as distance from origin, angle. This makes
            // proptest always generate circles outside the origin.
            circle_dist in 1.0..10.0f64,
            circle_ang in 0.0..100.0f64,
            // What point in the circle?
            chosen_point_rad_percent in 0.0..0.99f64,
            chosen_point_angle in 0.0..100.0f64,
        ) {
            let circle_dist_from_origin = radius + circle_dist + 0.1;
            // Center of the circle.
            let (c_x, c_y) = {
                let (dx, dy) = (circle_ang.cos(), circle_ang.sin());
                (x + dx * circle_dist_from_origin, y + dy * circle_dist_from_origin)
            };

            let circ = Circle::new(V2::new(c_x, c_y), radius).expect("Should work");
            let chosen_r = radius * chosen_point_rad_percent;
            let (target_x, target_y) = (c_x + chosen_r * chosen_point_angle.cos(), c_y + chosen_r * chosen_point_angle.sin());
            let target_dist = ((x - target_x).powi(2) + (y - target_y).powi(2)).sqrt();
            let (dx, dy) = ((target_x - x) / target_dist, (target_y - y) / target_dist);
            let hitting_ray = Ray::new(V2::new(x, y), V2::new(dx, dy), target_dist);
            let missing_ray = Ray::new(V2::new(x, y), V2::new(-dx, -dy), target_dist);
            let missing_test = ray_circle_test(&missing_ray, &circ);
            prop_assert!(missing_test.is_none(), "Casting to x={} y={}, {:?} {:?} {:?}", target_x, target_y, circ, missing_ray, missing_test);
            let hit_test = ray_circle_test(&hitting_ray, &circ);
            prop_assert!(hit_test.is_some(), "Casting to x={} y={}, {:?} {:?} {:?}", target_x, target_y, circ, hitting_ray, hit_test);
            let hit_data = hit_test.unwrap();
            prop_assert!(!hit_data.inside);
            prop_assert!(hit_data.normal.is_some());

            let hit_point_dist = hit_data.point.distance(&V2::new(c_x, c_y));
            assert!((hit_point_dist - radius).abs()  < 0.01, "{}", hit_point_dist);
            let origin_dist = hit_data.point.distance(&V2::new(x, y));
            assert!(origin_dist < circle_dist_from_origin);

            // It is difficult to precisely test normals, but the direction of
            // the normal should be the same as that of the vector from the
            // center of the sphere, and the dot product of the normal with the
            // ray's direction should be negative.
            let expected_normal = V2::new(c_x - hit_data.point.x, c_y - hit_data.point.y).normalize();
            let dnorm = expected_normal.dot(&hit_data.normal.unwrap());
            assert!(dnorm > 0.99, "{:?} dot is {}", expected_normal, dnorm);

            // By inverting our ray so that we cast out from the chosen point
            // inside the sphere, we can test whether or not rays inside
            // pointing out work.
            let inside_ray = Ray::new(V2::new(target_x, target_y), V2::new(dx, dy), 1.0);
            let inside_test = ray_circle_test(&inside_ray, &circ);
            prop_assert!(inside_test.is_some(), "{:?} {:?} {:?}",circ, inside_ray, inside_test);
            let inside_test = inside_test.unwrap();
            assert!(inside_test.inside);
            assert!(inside_test.normal.is_none());
        }
    }
}
