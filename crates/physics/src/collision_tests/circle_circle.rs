//! Test collision between two circles.
use crate::*;

pub(crate) fn circle_circle_test(c1: &Circle, c2: &Circle) -> bool {
    let cent1 = c1.get_center();
    let cent2 = c2.get_center();
    let dist_squared = (cent1.x - cent2.x).powi(2) + (cent1.y - cent2.y).powi(2);
    // Avoid square root, which is generally very slow.
    let touching_dist_squared = (c1.get_radius() + c2.get_radius()).powi(2);
    dist_squared < touching_dist_squared
}

#[cfg(test)]
mod tests {
    use super::*;

    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100000))]
        #[test]
        fn test_overlap(x1 in -1000.0..=1000.0f64,
            y1 in -1000.0..=1000.0f64,
            x2 in -1000.0..=1000.0f64,
            y2 in -1000.0..=1000.0f64,
            // Lets us make spheres of different sizes relative to each other.
            dist_percent in 0.1..=0.9f64,
        ) {
            let total_radius = ((x2-x1).powi(2)+(y2-y1).powi(2)).sqrt();
            let r1 = total_radius * dist_percent + 1.0;
            let r2 = total_radius * (1.0f64 - dist_percent) + 1.0;
            let circle1 = Circle::new(V2::new(x1, y1), r1).expect("Should succeed");
            let circle2 = Circle::new(V2::new(x2, y2), r2).expect("Should succeed");
            prop_assert!(circle_circle_test(&circle1, &circle2), "{:?} {:?}", circle1, circle2);
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100000))]
        #[test]
        fn test_not_colliding(x in -1000.0..=1000.0f64,
            y in -1000.0..=1000.0f64,
            angle in 0.0..=100.0f64,
            total_radius in 1.0..1000.0f64,
            gap_size in 2.0..1000.0f64,
            rad_percent in 0.1..=0.9f64,
        ) {
            let dist = total_radius + gap_size;
            let r1 = total_radius*rad_percent;
            let r2 = total_radius*(1.0f64 - rad_percent);
            let dx = angle.cos();
            let dy = angle.sin();
            let x2 = x + dx * dist;
            let y2 = y + dy * dist;
            let circle1 = Circle::new(V2::new(x, y), r1).expect("Should succeed");
            let circle2 = Circle::new(V2::new(x2, y2), r2).expect("Should succeed");
            prop_assert!(!circle_circle_test(&circle1, &circle2), "{:?} {:?}", circle1, circle2);
        }
    }
}
