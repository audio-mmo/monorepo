//! An axis-aligned bounding box.
use anyhow::{anyhow, Result};
use num::traits::Num;

use crate::*;

/// An axis-aligned bounding box is specified by the lower left point and a width/height vector.
#[derive(Debug, Copy, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct Aabb<T> {
    p1: V2<T>,
    /// width-height
    wh: V2<T>,
}

impl<T: Num + Copy> Aabb<T> {
    pub fn get_p1(&self) -> V2<T> {
        self.p1
    }

    pub fn get_p2(&self) -> V2<T> {
        self.p1 + self.wh
    }

    pub fn get_width(&self) -> T {
        self.wh.x
    }

    pub fn get_height(&self) -> T {
        self.wh.y
    }
}

impl<T: Num + Copy + std::cmp::PartialOrd> Aabb<T> {
    pub fn from_points(p1: V2<T>, p2: V2<T>) -> Result<Aabb<T>> {
        if p1.x > p2.x {
            return Err(anyhow!("p2.x > p1.x"));
        }
        if p1.y > p2.y {
            return Err(anyhow!("p1.y > p2.y"));
        }

        let wh = p2 - p1;
        Ok(Aabb { p1, wh })
    }
}

impl<T: Num + Copy> Aabb<T>
where
    f64: From<T>,
{
    pub fn get_half_width(&self) -> f64 {
        f64::from(self.get_width()) / 2.0
    }

    pub fn get_half_height(&self) -> f64 {
        f64::from(self.get_height()) / 2.0
    }

    pub fn get_center(&self) -> V2<f64> {
        V2 {
            x: f64::from(self.p1.x) + self.get_half_width(),
            y: f64::from(self.p1.y) + self.get_half_height(),
        }
    }

    /// get the squared distance to a specific point.
    pub fn distance_to_point_squared(&self, point: &V2<f64>) -> f64 {
        // The closest point on a box to a point is the clamped value of the point itself.
        let p1 = self.get_p1();
        let p2 = self.get_p2();
        let p1x: f64 = p1.x.into();
        let p1y = p1.y.into();
        let p2x = p2.x.into();
        let p2y = p2.y.into();
        let x = point.x.clamp(p1x, p2x);
        let y = point.y.clamp(p1y, p2y);
        (point.x - x).powi(2) + (point.y - y).powi(2)
    }

    pub fn distance_to_point(&self, point: &V2<f64>) -> f64 {
        self.distance_to_point_squared(point).sqrt()
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
