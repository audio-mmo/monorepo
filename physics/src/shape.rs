//! A shape is one of the concrete shape types behind an enum for collision
//! detection.
use crate::*;

#[derive(Debug)]
pub enum Shape {
    Aabb(Aabb),
    Circle(Circle),
    Ray(Ray),
}

impl Shape {
    pub fn get_bounding_box(&self) -> Aabb {
        match self {
            Shape::Aabb(ref a) => a.get_bounding_box(),
            Shape::Circle(ref c) => c.get_bounding_box(),
            Shape::Ray(ref r) => r.get_bounding_box(),
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

impl From<Ray> for Shape {
    fn from(other: Ray) -> Shape {
        Shape::Ray(other)
    }
}
