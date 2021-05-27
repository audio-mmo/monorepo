use crate::rays::RaycastPointIterator;

#[derive(Debug, PartialEq)]
pub struct Ray {
    pub x: f64,
    pub y: f64,
    pub dx: f64,
    pub dy: f64,
    pub length: f64,
}

impl Ray {
    pub fn from_angle(x:f64, y:f64, length:f64, theta:f64) -> Ray {
        Ray::new(x, y, theta.cos(), theta.sin(), length)
    }

    pub fn new(x: f64, y: f64, dx: f64, dy: f64, length: f64) -> Ray {
        Ray {
            x,
            y,
            dx,
            dy,
            length,
        }
    }

    pub fn raycast(&self) -> RaycastPointIterator {
        RaycastPointIterator::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn from_angle_tests() {
        let correct = Ray::new(0.0, 0.0, 1.0, 0.0, 1.0);
        let test = Ray::from_angle(0.0, 0.0, 1.0, 0.0);
        assert_eq!(test, correct);
    }
}
