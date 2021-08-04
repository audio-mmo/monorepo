//! Test distance between an AABB and a circle.
use crate::*;

pub(crate) fn aabb_circle_test(aabb: &Aabb, circle: &Circle) -> bool {
    let dist = aabb.distance_to_point_squared(circle.get_center());
    dist < circle.get_radius().powi(2)
}

#[cfg(test)]
mod tests {
    use super::*;

    use proptest::prelude::*;

    // Given a box with a given center, width, and height, and a given circle
    // radius, build some test cases which we know overlap, and some which we
    // don't.  This test tests that, when the sphere is adjacent to the side of
    // the box, it can't collide.  In effect, this covers all cases because we
    // aren't using the minkowski summ currently: the sphere is always
    // "adjacent" to some side of the box, even when at a corner.
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1000000))]
        #[test]
        fn test_overlaps_edges(
            x in -1000.0..=1000.0f64,
            y in -1000.0..1000.0f64,
            width in 1.0..100.0f64,
            height in 1.0..100.0f64,
            // used to specify where on the edge of the box we're going to take
            // the circle's center.
            width_percent in 0.01..0.99f64,
            height_percent in 0.01..0.99f64,
            circle_radius in 1.0..100.0f64,
            circle_dist_percent in 0.1..=0.9f64,
        ) {
            let circle_width_dist = (width + circle_radius)*circle_dist_percent;
            let circle_height_dist = (height + circle_radius) * circle_dist_percent;
            let centers = [
                V2::new(x + circle_width_dist, y + height * height_percent),
                V2::new(x - circle_width_dist, y + height * height_percent),
                V2::new(x + width * width_percent, y + circle_height_dist),
                V2::new(x - width * width_percent, y - circle_height_dist),
            ];

            let aabb = Aabb::from_points(V2::new(x - width, y - height), V2::new(x + width, y + height)).expect("Should succeed");
            for center in centers.iter().cloned() {
                let circle = Circle::new(center, circle_radius).expect("Should succeed");
                prop_assert!(aabb_circle_test(&aabb, &circle), "{:?} {:?} {}", aabb, circle, aabb.distance_to_point_squared(&center));
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1000000))]
        #[test]
        fn test_not_overlapping(
            x in -1000.0..=1000.0f64,
            y in -1000.0..1000.0f64,
            width in 1.0..100.0f64,
            height in 1.0..100.0f64,
            // used to specify where on the edge of the box we're going to take
            // the circle's center.
            width_percent in 0.01..0.99f64,
            height_percent in 0.01..0.99f64,
            circle_radius in 1.0..100.0f64,
            gap in 0.1..100.0f64,
        ) {
            let circle_width_dist = width + circle_radius + gap;
            let circle_height_dist = height + circle_radius + gap;
            let centers = [
                V2::new(x + circle_width_dist, y + height * height_percent),
                V2::new(x - circle_width_dist, y + height * height_percent),
                V2::new(x + width * width_percent, y + circle_height_dist),
                V2::new(x - width * width_percent, y - circle_height_dist),
            ];

            let aabb = Aabb::from_points(V2::new(x - width, y - height), V2::new(x + width, y + height)).expect("Should succeed");
            for center in centers.iter().cloned() {
                let circle = Circle::new(center, circle_radius).expect("Should succeed");
                prop_assert!(!aabb_circle_test(&aabb, &circle), "{:?} {:?} {}", aabb, circle, aabb.distance_to_point_squared(&center));
            }
        }
    }
}
