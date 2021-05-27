//An implementation of the algorithm found at
//http://playtechs.blogspot.com/2007/03/raytracing-on-grid.html

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
        if self.n > 0 {
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
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::rays::*;
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

    fn test_circle(cx: f64, cy: f64, radius: f64) {
        for theta in 0..6300 {
            let theta = (theta as f64) * 0.001;
            println!("theta={}", theta);
            let (unit_x, unit_y) = (theta.cos(), theta.sin());
            println!("Unit {}, {}", unit_x, unit_y);
            let (mut x_inc, mut y_inc) = (0, 0);
            if unit_x > 0.0 {
                x_inc = 1;
            } else if unit_x < 0.0 {
                x_inc = -1;
            }
            if unit_y > 0.0 {
                y_inc = 1;
            } else if unit_y < 0.0 {
                y_inc = -1;
            }
            let test: HashSet<(i64, i64)> =
                Ray::from_angle(cx, cy, radius, theta).raycast().collect();
            let (cx0, cy0) = (cx, cy);
            let (cx1, cy1) = (cx + unit_x * radius, cy + unit_y * radius);
            let (cdx, cdy) = (cx1 - cx0, cy1 - cy0);
            let mut correct: HashSet<(i64, i64)> = HashSet::new();
            let (tile_x_min, tile_y_min) = (
                cx0.floor().min(cx1.floor()) as i64,
                cy0.floor().min(cy1.floor()) as i64,
            );
            let (tile_x_max, tile_y_max) = (
                cx1.floor().max(cx0.floor()) as i64,
                cy1.floor().max(cy0.floor()) as i64,
            );
            for tx in tile_x_min..=tile_x_max {
                for ty in tile_y_min..=tile_y_max {
                    let (tx1, ty1) = (tx as f64 + 0.5, ty as f64 + 0.5);
                    let (tdx, tdy) = (tx1 - cx0, ty1 - cy0);
                    //Project (tdx, tdy) onto (cdx, cdy)
                    //k will be a scalar of (cdx, cdy)
                    let mut k = (tdx * cdx + tdy * cdy) / (cdx * cdx + cdy * cdy);
                    //If k is greater than 1 or less than 0 then the closest point to the center of the tile is the corresponding end of the ray
                    k = k.clamp(0.0, 1.0);
                    let (px, py) = (cx0 + k * cdx, cy0 + k * cdy);
                    if px.floor() as i64 == tx && py.floor() as i64 == ty {
                        correct.insert((tx, ty));
                        //We need to make sure none of the tiles the algorithm will use are skipped
                        let (prop_x, prop_y);
                        if unit_x > 0.0 && tx + 1 <= tile_x_max {
                            prop_x = (px.ceil() - px) / unit_x;
                        } else if unit_x < 0.0 && tx - 1 >= tile_x_min {
                            prop_x = (px.floor() - px) / unit_x;
                        } else {
                            prop_x = f64::INFINITY;
                        }
                        if unit_y > 0.0 && ty + 1 <= tile_y_max {
                            prop_y = (py.ceil() - py) / unit_y;
                        } else if unit_y < 0.0 && ty - 1 >= tile_y_min {
                            prop_y = (py.floor() - py) / unit_y;
                        } else {
                            prop_y = f64::INFINITY;
                        }
                        if prop_y < prop_x && prop_y != f64::INFINITY {
                            correct.insert((tx, ty + y_inc));
                        } else if prop_x != f64::INFINITY {
                            correct.insert((tx + x_inc, ty));
                        }
                    }
                }
            }
            assert_eq!(test, correct);
        }
    }
}
