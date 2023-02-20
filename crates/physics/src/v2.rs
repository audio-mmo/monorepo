//! A 2-dimensional vector/point.
use num::Num;

#[derive(Debug, Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct V2<T> {
    pub x: T,
    pub y: T,
}

impl<T: Num> V2<T> {
    pub const fn new(x: T, y: T) -> Self {
        V2 { x, y }
    }
}

impl<T> V2<T>
where
    T: Num + Copy,
    f64: From<T>,
{
    pub fn length_squared(&self) -> f64 {
        let x: f64 = self.x.into();
        let y: f64 = self.y.into();
        x * x + y * y
    }

    pub fn length(&self) -> f64 {
        self.length_squared().sqrt()
    }

    #[must_use = "This function doesn't modify the vector in place"]
    pub fn normalize(self) -> V2<f64> {
        let l = self.length();
        V2 {
            x: f64::from(self.x) / l,
            y: f64::from(self.y) / l,
        }
    }

    pub fn dot(&self, other: &V2<T>) -> f64 {
        let sx: f64 = self.x.into();
        let sy: f64 = self.y.into();
        let ox: f64 = other.x.into();
        let oy: f64 = other.y.into();

        sx * ox + sy * oy
    }

    pub fn distance_squared(&self, other: &V2<T>) -> f64 {
        let x1: f64 = self.x.into();
        let y1: f64 = self.y.into();
        let x2: f64 = other.x.into();
        let y2: f64 = other.y.into();
        (x2 - x1).powi(2) + (y2 - y1).powi(2)
    }

    pub fn distance(&self, other: &V2<T>) -> f64 {
        self.distance_squared(other).sqrt()
    }
}

impl<T: Num> std::ops::Add for V2<T> {
    type Output = V2<T>;

    fn add(self, rhs: V2<T>) -> V2<T> {
        V2 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl<T: Num + Copy> std::ops::AddAssign for V2<T> {
    fn add_assign(&mut self, rhs: Self) {
        self.x = self.x + rhs.x;
        self.y = self.y + rhs.y;
    }
}

impl<T: Copy> std::ops::Mul<f64> for V2<T>
where
    f64: From<T>,
{
    type Output = V2<f64>;

    fn mul(self, rhs: f64) -> Self::Output {
        V2 {
            x: f64::from(self.x) * rhs,
            y: f64::from(self.y) * rhs,
        }
    }
}

impl<T: Copy> std::ops::Div<f64> for V2<T>
where
    f64: From<T>,
{
    type Output = V2<f64>;

    fn div(self, rhs: f64) -> Self::Output {
        V2 {
            x: f64::from(self.x) / rhs,
            y: f64::from(self.y) / rhs,
        }
    }
}

impl<T: std::ops::Neg> std::ops::Neg for V2<T> {
    type Output = V2<<T as std::ops::Neg>::Output>;

    fn neg(self) -> Self::Output {
        V2 {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl<T: Num> std::ops::Sub for V2<T> {
    type Output = V2<T>;

    fn sub(self, rhs: Self) -> Self::Output {
        V2 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl<T: Num + Copy> std::ops::SubAssign for V2<T> {
    fn sub_assign(&mut self, rhs: Self) {
        self.x = self.x - rhs.x;
        self.y = self.y - rhs.y;
    }
}
