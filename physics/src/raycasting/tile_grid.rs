//! An implementation of the algorithm found at
//! http://playtechs.blogspot.com/2007/03/raytracing-on-grid.html
use crate::*;

pub struct TileGridRaycastPointIterator {
    dx: f64,
    dy: f64,
    x: i64,
    y: i64,
    n: i64,
    x_inc: i64,
    y_inc: i64,
    error: f64,
}

impl TileGridRaycastPointIterator {
    pub fn new(ray: &Ray) -> TileGridRaycastPointIterator {
        let (x0, y0) = (ray.origin.x, ray.origin.y);
        let (x1, y1) = (
            x0 + ray.direction.x * ray.length,
            y0 + ray.direction.y * ray.length,
        );
        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();
        let x = x0.floor() as i64;
        let y = y0.floor() as i64;
        let mut n: i64 = 1;
        let x_inc: i64;
        let y_inc: i64;
        let mut error: f64;

        if dx == 0.0 {
            x_inc = 0;
            error = f64::INFINITY;
        } else if x1 > x0 {
            x_inc = 1;
            n += x1.floor() as i64 - x;
            error = (x0.floor() + 1.0 - x0) * dy;
        } else {
            x_inc = -1;
            n += x - x1.floor() as i64;
            error = (x0 - x0.floor()) * dy;
        }

        if dy == 0.0 {
            y_inc = 0;
            error -= f64::INFINITY;
        } else if y1 > y0 {
            y_inc = 1;
            n += y1.floor() as i64 - y;
            error -= (y0.floor() + 1.0 - y0) * dx;
        } else {
            y_inc = -1;
            n += y - y1.floor() as i64;
            error -= (y0 - y0.floor()) * dx;
        }

        TileGridRaycastPointIterator {
            dx,
            dy,
            x,
            y,
            n,
            x_inc,
            y_inc,
            error,
        }
    }
}

impl Iterator for TileGridRaycastPointIterator {
    type Item = (i64, i64);
    fn next(&mut self) -> Option<Self::Item> {
        if self.n <= 0 {
            return None;
        }

        self.n -= 1;
        let (x, y) = (self.x, self.y);
        if self.error > 0.0 {
            self.y += self.y_inc;
            self.error -= self.dx;
        } else {
            self.x += self.x_inc;
            self.error += self.dy;
        }
        Some((x, y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_length_one_simple() {
        let directions = vec![(1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0)];
        for (x, y) in directions.into_iter() {
            let test = Ray::new(V2::new(0.0, 0.0), V2::new(x, y), 1.0)
                .raycast()
                .collect::<Vec<_>>();
            let correct = vec![(0, 0), (x as i64, y as i64)];
            assert_eq!(test, correct);
        }
    }

    #[test]
    fn test_length_zero_simple() {
        let directions = vec![(1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0)];
        let correct = [(0i64, 0i64)];
        for (x, y) in directions.into_iter() {
            let test = Ray::new(V2::new(0.0, 0.0), V2::new(x, y), 0.0)
                .raycast()
                .collect::<Vec<_>>();
            assert_eq!(test, correct);
        }
    }
}
