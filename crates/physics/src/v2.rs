//! A 2-dimensional vector/point.

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct V2 {
    pub x: f64,
    pub y: f64,
}

impl V2 {
    pub const fn new(x: f64, y: f64) -> V2 {
        V2 { x, y }
    }

    pub fn length_squared(&self) -> f64 {
        self.x * self.x + self.y * self.y
    }

    pub fn length(&self) -> f64 {
        self.length_squared().sqrt()
    }

    #[must_use = "This function doesn't modify the vector in place"]
    pub fn normalize(self) -> V2 {
        let l = self.length();
        V2 {
            x: self.x / l,
            y: self.y / l,
        }
    }

    pub fn dot(&self, other: &V2) -> f64 {
        self.x * other.x + self.y * other.y
    }

    pub fn distance_squared(&self, other: &V2) -> f64 {
        (self.x - other.x).powi(2) + (self.y - other.y).powi(2)
    }

    pub fn distance(&self, other: &V2) -> f64 {
        self.distance_squared(other).sqrt()
    }
}

impl std::ops::Add for V2 {
    type Output = V2;

    fn add(self, rhs: V2) -> V2 {
        V2 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl std::ops::AddAssign for V2 {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl std::ops::Mul<f64> for V2 {
    type Output = V2;

    fn mul(self, rhs: f64) -> Self::Output {
        V2 {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl std::ops::Mul<f32> for V2 {
    type Output = V2;

    fn mul(self, rhs: f32) -> Self::Output {
        self * (rhs as f64)
    }
}

impl std::ops::MulAssign<f64> for V2 {
    fn mul_assign(&mut self, rhs: f64) {
        *self = *self * rhs;
    }
}

impl std::ops::MulAssign<f32> for V2 {
    fn mul_assign(&mut self, rhs: f32) {
        *self *= rhs as f64;
    }
}

impl std::ops::Div<f64> for V2 {
    type Output = V2;

    fn div(self, rhs: f64) -> Self::Output {
        V2 {
            x: self.x / rhs,
            y: self.y / rhs,
        }
    }
}

impl std::ops::DivAssign<f64> for V2 {
    fn div_assign(&mut self, rhs: f64) {
        *self = *self / rhs;
    }
}

impl std::ops::Div<f32> for V2 {
    type Output = V2;

    fn div(self, rhs: f32) -> Self::Output {
        self / (rhs as f64)
    }
}

impl std::ops::DivAssign<f32> for V2 {
    fn div_assign(&mut self, rhs: f32) {
        *self /= rhs as f64;
    }
}

impl std::ops::Neg for V2 {
    type Output = V2;

    fn neg(self) -> Self::Output {
        V2 {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl std::ops::Sub for V2 {
    type Output = V2;

    fn sub(self, rhs: Self) -> Self::Output {
        self + -rhs
    }
}

impl std::ops::SubAssign for V2 {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}
