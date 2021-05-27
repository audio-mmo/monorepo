//! An implementation of the algorithm found at
//! http://playtechs.blogspot.com/2007/03/raytracing-on-grid.html

use crate::rays::Ray;

pub struct RaycastPointIterator {
    dx: f64,
    dy: f64,
    x: i64,
    y: i64,
    n: i64,
    x_inc: i64,
    y_inc: i64,
    error: f64,
}

impl RaycastPointIterator {
    pub fn new(ray: &Ray) -> RaycastPointIterator {
        let (x0, y0) = (ray.x, ray.y);
        let (x1, y1) = (x0 + ray.dx * ray.length, y0 + ray.dy * ray.length);
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

        RaycastPointIterator {
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

impl Iterator for RaycastPointIterator {
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

    use std::collections::HashSet;

    #[test]
    fn test_length_one_simple() {
        let directions = vec![(1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0)];
        for (x, y) in directions.into_iter() {
            let test = Ray::new(0.0, 0.0, x, y, 1.0).raycast().collect::<Vec<_>>();
            let correct = vec![(0, 0), (x as i64, y as i64)];
            assert_eq!(test, correct);
        }
    }

    #[test]
    fn test_length_zero_simple() {
        let directions = vec![(1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0)];
        let correct = [(0i64, 0i64)];
        for (x, y) in directions.into_iter() {
            let test = Ray::new(0.0, 0.0, x, y, 0.0).raycast().collect::<Vec<_>>();
            assert_eq!(test, correct);
        }
    }

    #[test]
    fn unit_circle() {
        test_circle(0.0, 0.0, 1.0);
    }

    #[test]
    fn large_circle() {
        test_circle(0.0, 0.0, 2.0);
    }

    /// Iterator from start to stop by step. Not safe for use in non-test code.
    fn float_iter(start: f64, stop: f64, step: f64) -> impl Iterator<Item = f64> {
        let count = ((stop - start) / step) as u64;
        (0..count)
            .into_iter()
            .map(move |i| start + (i as f64 * step))
    }

    /// Get the bounding box of a ray (todo: this should use a proper aabb
    /// type).
    fn bounding_box(r: Ray) -> ((i64, i64), (i64, i64)) {
        // Work out the largest bounding box.
        let x0 = r.x;
        let x1 = r.x + r.dx * r.length;
        let y0 = r.y;
        let y1 = r.y + r.dy * r.length;
        let xmin = x0.min(x1);
        let xmax = x0.max(x1);
        let ymin = y0.min(y1);
        let ymax = y0.max(y1);
        (
            (xmin.floor() as i64, ymin.floor() as i64),
            (xmax.ceil() as i64, ymax.ceil() as i64),
        )
    }

    fn raycast_slow(r: Ray) -> HashSet<(i64, i64)> {
        let mut ret = HashSet::new();
        let ((x1, y1), (x2, y2)) = bounding_box(r);

        ret.insert((r.x.floor() as i64, r.y.floor() as i64));

        for tile_x in x1..=x2 {
            for tile_y in y1..=y2 {
                let center_x = tile_x as f64 + 0.5;
                let center_y = tile_y as f64 + 0.5;
                let delta_x = center_x - r.x;
                let delta_y = center_y - r.y;
                // Get `t`, the distance along the ray closest to the tile.
                // Note that the ray's dx and dy are already a unit vector.
                let proj_t = (delta_x * r.dx + delta_y * r.dy).clamp(0.0, r.length);
                let closest_x = r.x + r.dx * proj_t;
                let closest_y = r.y + proj_t * r.dy;
                // Now it's the standard box intersection test.
                if tile_x as f64 <= closest_x
                    && closest_x <= (tile_x + 1) as f64
                    && tile_y as f64 <= closest_y
                    && closest_y <= (tile_y + 1) as f64
                {
                    ret.insert((tile_x, tile_y));
                }
            }
        }

        ret
    }

    fn test_circle(cx: f64, cy: f64, radius: f64) {
        for theta in float_iter(0.0, std::f64::consts::PI * 2.0, 0.01) {
            let (unit_x, unit_y) = (theta.cos(), theta.sin());
            let r = Ray {
                x: cx,
                y: cy,
                dx: unit_x,
                dy: unit_y,
                length: radius,
            };
            let mut casted = RaycastPointIterator::new(&r).collect::<Vec<_>>();
            // casted is unsorted because we dont' know which way the ray goes.
            casted.sort_unstable();
            let mut expected = raycast_slow(r).into_iter().collect::<Vec<_>>();
            // Expected is unsorted because it's checking every tile.
            expected.sort_unstable();
            assert_eq!(
                casted,
                expected,
                "angle={} unit_x={} unit_y={}",
                theta * 180.0 / std::f64::consts::PI,
                unit_x,
                unit_y
            );
        }
    }
}
