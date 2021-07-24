//! A shape is one of the concrete shape types behind an enum for collision
//! detection.
use crate::*;

#[derive(Debug)]
pub enum Shape {
    Aabb(Aabb),
    Circle(Circle),
}

impl Shape {
    pub fn get_bounding_box(&self) -> Aabb {
        match self {
            Shape::Aabb(ref a) => a.get_bounding_box(),
            Shape::Circle(ref c) => c.get_bounding_box(),
        }
    }

    /// Test if this shape collides with another.
    pub fn collides_with(&self, other: &Shape) -> bool {
        use crate::collision_tests::*;
        use Shape::*;

        match (self, other) {
            (Aabb(ref a), Aabb(ref b)) => aabb_aabb_test(a, b),
            (Circle(ref a), Circle(ref b)) => circle_circle_test(a, b),
            (Aabb(ref a), Circle(ref b)) | (Circle(ref b), Aabb(ref a)) => aabb_circle_test(a, b),
        }
    }
}

impl From<Aabb> for Shape {
    fn from(other: Aabb) -> Shape {
        Shape::Aabb(other)
    }
}

impl From<Circle> for Shape {
    fn from(other: Circle) -> Shape {
        Shape::Circle(other)
    }
}