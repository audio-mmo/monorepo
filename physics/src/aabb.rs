//! An axis-aligned bounding box.
use anyhow::Result;

use crate::*;

/// An axis-aligned bounding box is specified by 2 points `p1` and `p2`, such
/// that `p1.x <= p2.x && p1.y <= p2.y`.
///
/// It is tempting to make the authoritative representation be a center and
/// half-width pair, but this doesn't work: floating point inaccuracies mean
/// that when using this as a bounding box, the box can be slightly too small.
/// There are tests in `ray.rs`, for example, which fail because the bounding
/// box *doesn't contain the starting point of the ray* with such a
/// representation.  
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct Aabb {
    p1: V2,
    p2: V2,
}

impl Aabb {
    pub fn from_points(p1: V2, p2: V2) -> Result<Aabb> {
        if p1.x > p2.x {
            return Err(anyhow!("p2.x > p1.x"));
        }
        if p1.y > p2.y {
            return Err(anyhow!("p1.y > p2.y"));
        }

        Ok(Aabb { p1, p2 })
    }

    pub fn get_p1(&self) -> &V2 {
        &self.p1
    }

    pub fn get_p2(&self) -> &V2 {
        &self.p2
    }

    pub fn get_width(&self) -> f64 {
        self.p2.x - self.p1.x
    }

    pub fn get_height(&self) -> f64 {
        self.p2.y - self.p1.y
    }

    pub fn get_half_width(&self) -> f64 {
        self.get_width() / 2.0
    }

    pub fn get_half_height(&self) -> f64 {
        self.get_height() / 2.0
    }

    pub fn get_center(&self) -> V2 {
        V2 {
            x: self.p1.x + self.get_half_width(),
            y: self.p1.y + self.get_half_height(),
        }
    }

    pub fn get_bounding_box(&self) -> Aabb {
        *self
    }

    /// get the squared distance to a specific point.
    pub fn distance_to_point_squared(&self, point: &V2) -> f64 {
        // The closest point on a box to a point is the clamped value of the point itself.
        let x = point.x.clamp(self.p1.x, self.p2.x);
        let y = point.y.clamp(self.p1.y, self.p2.y);
        (point.x - x).powi(2) + (point.y - y).powi(2)
    }

    pub fn distance_to_point(&self, point: &V2) -> f64 {
        self.distance_to_point_squared(point).sqrt()
    }

    /// Move the box.  This may slightly warp the dimensions of the box and
    /// should not be used on AABBs which don't represent objects (e.g. as the
    /// input to a spatial hash) becuase this movement can shrink the box
    /// slightly (so that e.g. the starting point of a ray is slightly outside
    /// the aabb).
    #[must_use = "This doesn't mutate the Aabb in place"]
    pub fn move_aabb(&self, new_center: &V2) -> Aabb {
        let half_dims: V2 = (self.p2 - self.p1) / 2.0;
        let p1 = *new_center - half_dims;
        let p2 = *new_center + half_dims;
        Aabb { p1, p2 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() -> Result<()> {
        let b = Aabb::from_points(V2::new(1.0, 1.0), V2::new(3.0, 5.0))?;
        approx::assert_relative_eq!(b.get_width(), 2.0);
        approx::assert_relative_eq!(b.get_height(), 4.0);
        approx::assert_relative_eq!(b.get_half_width(), 1.0);
        approx::assert_relative_eq!(b.get_half_height(), 2.0);
        Ok(())
    }

    #[test]
    fn move_aabb() {
        let aabb1 = Aabb::from_points(V2::new(1.0, 1.0), V2::new(3.0, 3.0)).unwrap();
        let aabb2 = aabb1.move_aabb(&V2::new(10.0, 5.0));
        assert_eq!(*aabb2.get_p1(), V2::new(9.0, 4.0));
        assert_eq!(*aabb2.get_p2(), V2::new(11.0, 6.0));
    }
}
