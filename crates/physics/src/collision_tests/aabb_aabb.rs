//! The AABB-AABB collision test.
use crate::*;

pub(crate) fn aabb_aabb_test(box1: &Aabb, box2: &Aabb) -> bool {
    let min_x = box1.get_p1().x.min(box2.get_p1().x);
    let max_x = box1.get_p2().x.max(box2.get_p2().x);
    let min_y = box1.get_p1().y.min(box2.get_p1().y);
    let max_y = box1.get_p2().y.max(box2.get_p2().y);
    let dist_x = max_x - min_x;
    let dist_y = max_y - min_y;

    dist_x <= (box1.get_width() + box2.get_width())
        && dist_y <= (box1.get_height() + box2.get_height())
}

#[cfg(test)]
mod tests {
    use super::*;

    use proptest::prelude::*;

    // A second implementation of a box-box collision algorithm that we know to be correct.
    fn test_oracle(b1: &Aabb, b2: &Aabb) -> bool {
        let not_touching_width = b1.get_width() + b2.get_width();
        let not_touching_height = b1.get_height() + b2.get_height();
        let min_x = b1.get_p1().x.min(b2.get_p1().x);
        let max_x = b1.get_p2().x.max(b2.get_p2().x);
        let min_y = b1.get_p1().y.min(b2.get_p1().y);
        let max_y = b1.get_p2().y.max(b2.get_p2().y);
        (max_x - min_x) < not_touching_width && (max_y - min_y) < not_touching_height
    }

    #[test]
    fn basic() -> anyhow::Result<()> {
        let b1 = Aabb::from_points(V2::new(0.0, 0.0), V2::new(2.0, 2.0))?;
        let b2 = Aabb::from_points(V2::new(1.0, 1.0), V2::new(3.0, 3.0))?;
        assert!(aabb_aabb_test(&b1, &b2));
        Ok(())
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100000))]
        #[test]
        fn fuzz(x1 in -1000.0..=1000.0f64,
            x2 in -1000.0..=1000.0f64,
            y1 in -1000.0..=1000.0f64,
            y2 in -1000.0..=1000.0f64,
            x3 in -1000.0..=1000.0f64,
            x4 in -1000.0..=1000.0f64,
            y3 in -1000.0..=1000.0f64,
            y4 in -1000.0..=1000.0f64,
        ) {
            let box1 = {
                let xmin = x1.min(x2);
                let xmax = x1.max(x2);
                let ymin = y1.min(y2);
                let ymax = y1.max(y2);
                Aabb::from_points(V2::new(xmin, ymin), V2::new(xmax, ymax)).expect("Should never fail")
            };

            let box2 = {
                let xmin = x3.min(x4);
                let xmax = x3.max(x4);
                let ymin = y3.min(y4);
                let ymax = y3.max(y4);
                Aabb::from_points(V2::new(xmin, ymin), V2::new(xmax, ymax)).expect("Shouldn't fail")
            };

            prop_assert_eq!(aabb_aabb_test(&box1, &box2), test_oracle(&box1, &box2), "{:?} {:?}", box1, box2);
        }
    }

    // Does swapping the arguments always yield the same result?
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100000))]
        #[test]
        fn test_symmetry(x1 in -1000.0..=1000.0f64,
            x2 in -1000.0..=1000.0f64,
            y1 in -1000.0..=1000.0f64,
            y2 in -1000.0..=1000.0f64,
            x3 in -1000.0..=1000.0f64,
            x4 in -1000.0..=1000.0f64,
            y3 in -1000.0..=1000.0f64,
            y4 in -1000.0..=1000.0f64,
        ) {
            let box1 = {
                let xmin = x1.min(x2);
                let xmax = x1.max(x2);
                let ymin = y1.min(y2);
                let ymax = y1.max(y2);
                Aabb::from_points(V2::new(xmin, ymin), V2::new(xmax, ymax)).expect("Should never fail")
            };

            let box2 = {
                let xmin = x3.min(x4);
                let xmax = x3.max(x4);
                let ymin = y3.min(y4);
                let ymax = y3.max(y4);
                Aabb::from_points(V2::new(xmin, ymin), V2::new(xmax, ymax)).expect("Shouldn't fail")
            };

            prop_assert_eq!(aabb_aabb_test(&box1, &box2), aabb_aabb_test(&box2, &box1), "{:?} {:?}", box1, box2);
        }
    }
}
