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

    pub fn raycast(&self) -> RaycastPointIterator {
        RaycastPointIterator::new(self)
    }

    pub fn get_aabb(&self) -> Aabb {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_angle_tests() {
        let correct = Ray::new(V2::new(0.0, 0.0), V2::new(1.0, 0.0), 1.0);
        let test = Ray::from_angle(V2::new(0.0, 0.0), 1.0, 0.0);
        assert_eq!(test, correct);
    }
}
