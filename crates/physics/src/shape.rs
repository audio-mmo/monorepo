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

    pub fn raycast(&self, ray: &Ray) -> Option<RaycastingResult> {
        match self {
            Shape::Circle(ref c) => crate::raycasting::ray_circle_test(ray, c),
            Shape::Aabb(ref a) => crate::raycasting::ray_aabb_test(ray, a),
        }
    }

    #[must_use = "This doesn't mutate the Shape in place"]
    pub fn move_shape(&self, new_center: &V2) -> Shape {
        match *self {
            Shape::Circle(ref c) => Shape::Circle(c.move_circle(new_center)),
            Shape::Aabb(ref a) => Shape::Aabb(a.move_aabb(new_center)),
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
