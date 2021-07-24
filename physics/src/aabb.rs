//! An axis-aligned bounding box.
use anyhow::Result;

use crate::*;

/// An axis-aligned bounding box is specified by 2 points `p1` and `p2`, such
/// that `p1.x <= p2.x && p1.y <= p2.y`.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct Aabb {
    p1: V2,
    p2: V2,
}

/// Specifies coroners of an AABB.
///
///This enum is ordered such that ordering an array containing its values
///produces a counterclockwise traversal starting at the bottom left coroner
///(e.g. p1).
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum AabbCorner {
    /// Lower left coroner, e.g. p1.
    Ll,
    /// Lower right coroner.
    Lr,
    /// Upper right coroner, e.g. p2.
    Ur,
    /// Upper left corner.
    Ul,
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

    pub fn get_corner(&self, corner: AabbCorner) -> V2 {
        use AabbCorner::*;

        match corner {
            Ll => *self.get_p1(),
            Ur => *self.get_p2(),
            Ul => V2::new(self.p1.x, self.p2.y),
            Lr => V2::new(self.p2.x, self.p1.y),
        }
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
}
