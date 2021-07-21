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
}
