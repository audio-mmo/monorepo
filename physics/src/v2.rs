//! A 2-dimensional vector/point.

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct V2 {
    pub x: f64,
    pub y: f64,
}

impl V2 {
    pub fn new(x: f64, y: f64) -> V2 {
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
}
