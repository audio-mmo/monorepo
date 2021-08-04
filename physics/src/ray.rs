use crate::*;

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct Ray {
    pub(crate) origin: V2,
    pub(crate) direction: V2,
    pub(crate) length: f64,
}

impl Ray {
    pub fn from_angle(origin: V2, length: f64, theta: f64) -> Ray {
        Ray::new(
            origin,
            V2 {
                x: theta.cos(),
                y: theta.sin(),
            },
            length,
        )
    }

    pub fn new(origin: V2, direction: V2, length: f64) -> Ray {
        Ray {
            origin,
            direction,
            length,
        }
    }

    /// Build a ray from a sourec point and a destination point.
    pub fn from_points(source: V2, target: V2) -> Ray {
        let length = source.distance(&target);
        let direction = V2::new(target.x - source.x, target.y - source.y).normalize();
        Ray::new(source, direction, length)
    }

    pub fn raycast(&self) -> TileGridRaycastPointIterator {
        TileGridRaycastPointIterator::new(self)
    }

    pub fn get_bounding_box(&self) -> Aabb {
        let x0 = self.origin.x;
        let y0 = self.origin.y;
        let x1 = self.origin.x + self.length * self.direction.x;
        let y1 = self.origin.y + self.direction.y * self.length;
        let p1 = V2 {
            x: x0.min(x1),
            y: y0.min(y1),
        };
        let p2 = V2 {
            x: x0.max(x1),
            y: y0.max(y1),
        };
        Aabb::from_points(p1, p2).expect("This internal logic should never fail")
    }

    /// Evaluate the ray at a given `t`.
    pub fn evaluate(&self, t: f64) -> V2 {
        V2::new(
            self.origin.x + self.direction.x * t,
            self.origin.y + self.direction.y * t,
        )
    }
}

#[cfg(test)]
mod tests {
    use approx::*;

    use super::*;

    #[test]
    fn from_angle_tests() {
        let correct = Ray::new(V2::new(0.0, 0.0), V2::new(1.0, 0.0), 1.0);
        let test = Ray::from_angle(V2::new(0.0, 0.0), 1.0, 0.0);
        assert_eq!(test, correct);
    }

    #[test]
    fn test_bounding_box() {
        let r = Ray::new(V2::new(1.0, 1.0), V2::new(1.0, 1.0).normalize(), 3.0);
        let aabb = r.get_bounding_box();
        assert_relative_eq!(aabb.get_p1().x, 1.0);
        assert_relative_eq!(aabb.get_p1().y, 1.0);
        assert_relative_eq!(aabb.get_p2().x, 3.121320343559643);
        assert_relative_eq!(aabb.get_p2().y, 3.121320343559643);
    }

    #[test]
    fn test_bounding_box_negative() {
        let r = Ray::new(V2::new(-1.0, -1.0), V2::new(-1.0, -1.0).normalize(), 3.0);
        let aabb = r.get_bounding_box();
        assert_relative_eq!(aabb.get_p1().x, -3.121320343559643);
        assert_relative_eq!(aabb.get_p1().y, -3.121320343559643);
        assert_relative_eq!(aabb.get_p2().x, -1.0);
        assert_relative_eq!(aabb.get_p2().y, -1.0);
    }
}
