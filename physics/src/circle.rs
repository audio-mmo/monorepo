//! A simple circle.
use anyhow::Result;

use crate::*;

pub struct Circle {
    center: V2,
    radius: f64,
}

impl Circle {
    pub fn new(center: V2, radius: f64) -> Result<Circle> {
        if radius < 0.0 {
            return Err(anyhow!("Radius must be positive"));
        }
        Ok(Circle { center, radius })
    }

    pub fn get_center(&self) -> &V2 {
        &self.center
    }

    pub fn get_radius(&self) -> f64 {
        self.radius
    }

    pub fn get_aabb(&self) -> Aabb {
        let p1 = V2::new(self.center.x - self.radius, self.center.y - self.radius);
        let p2 = V2::new(self.center.x + self.radius, self.center.y + self.radius);
        Aabb::from_points(p1, p2).expect("This internal logic should never fail")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use approx::*;

    #[test]
    fn test_aabb() -> Result<()> {
        let c = Circle::new(V2::new(1.0, 1.0), 2.0)?;
        let b = c.get_aabb();
        assert_relative_eq!(b.get_p1().x, -1.0);
        assert_relative_eq!(b.get_p1().y, -1.0);
        assert_relative_eq!(b.get_p2().x, 3.0);
        assert_relative_eq!(b.get_p2().y, 3.0);
        Ok(())
    }
}
