//! An axis-aligned bounding box.
use num::traits::Num;

use crate::errors::*;
use crate::morton::*;
use crate::*;

/// An axis-aligned bounding box is specified by the lower left point and a width/height vector.
///
/// Boxes can never be a single point or line.
#[derive(Debug, Copy, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct Aabb<T> {
    p1: V2<T>,
    /// width-height
    wh: V2<T>,
}

impl<T: Num + Copy> Aabb<T> {
    /// get a box from a point and a width/height pair. Note that this is unchecked: if wh is a zero vector or negative,
    /// behavior is undefined.
    fn from_point_wh(point: V2<T>, wh: V2<T>) -> Aabb<T> {
        Aabb { p1: point, wh }
    }

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
    pub fn from_points(p1: V2<T>, p2: V2<T>) -> Result<Aabb<T>, AabbError> {
        if p1.x >= p2.x || p1.y >= p2.y {
            return Err(AabbError::AabbInvalidDims);
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

impl Aabb<u16> {
    /// get an enclosing prefix for this box, which covers the lower left corners of all tiles which this box is in.
    ///
    /// If iterating over tiles that this box covers, then all tiles will have a lower left corner/position/etc which are subprefixes of the returned prefix.
    pub fn tile_prefix(&self) -> Result<MortonPrefix, AabbError> {
        let x = self.p1.x;
        let y = self.p1.y;
        let w = self.wh.x;
        let h = self.wh.y;
        let x2 = x.checked_add(w).ok_or(AabbError::AabbU16Overflow)?;
        let y2 = y.checked_add(h).ok_or(AabbError::AabbU16Overflow)?;
        Ok(MortonPrefix::from_code(MortonCode::encode(V2::new(x, y)))
            .merge(MortonPrefix::from_code(MortonCode::encode(V2::new(x2, y2)))))
    }

    /// get an iterator which will visit all tiles of this box, by rows of x starting from the minimum y:
    ///
    /// `(1, 1), (2, 1), ... (1, 2), (2, 2),...`
    ///
    /// Errors out if it is necessary to visit a tile which would be beyond `u16::MAX`, before iteration. The iterator
    /// itself is guaranteed never to overflow.
    fn iter_tiles(&self) -> Result<impl Iterator<Item = V2<u16>>> {
        let min_x = self.p1.x;
        let min_y = self.p1.y;
        let max_x = min_x
            .checked_add(self.wh.x - 1)
            .ok_or(AabbError::AabbU16Overflow)?;
        let max_y = min_y
            .checked_add(self.wh.y - 1)
            .ok_or(AabbError::AabbU16Overflow)?;

        let iter = (min_x..=max_x).flat_map(move |x| (min_y..=max_y).map(move |y| V2::new(x, y)));
        Ok(iter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use proptest::prelude::*;

    #[test]
    fn test_basic() -> crate::Result<()> {
        let b = Aabb::from_points(V2::new(1.0, 1.0), V2::new(3.0, 5.0))?;
        approx::assert_relative_eq!(b.get_width(), 2.0);
        approx::assert_relative_eq!(b.get_height(), 4.0);
        approx::assert_relative_eq!(b.get_half_width(), 1.0);
        approx::assert_relative_eq!(b.get_half_height(), 2.0);
        Ok(())
    }

    proptest! {
        #[test]
        fn test_morton(
            x in 0..(u16::MAX - 500),
            y in (0..u16::MAX - 500),
            w in 1..200u16,
            h in 1..200u16,
        ){
            let b = Aabb::from_point_wh(V2::new(x, y), V2::new(w, h));
            let prefix = b.tile_prefix().unwrap();
            let mut iterator = b.iter_tiles().unwrap();

            for x_i in x..(x + w) {
                for y_i in y..(y+h) {
                    prop_assert_eq!(iterator.next(), Some(V2::new(x_i, y_i)));
                    prop_assert!(prefix.contains_point(V2::new(x_i, y_i)));
                }
            }
        }
    }
}
