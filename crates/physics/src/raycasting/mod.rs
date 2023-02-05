use crate::*;

pub(crate) mod ray_aabb;
pub mod tile_grid;

/// The result of performing a raycasting test.
#[derive(Debug)]
pub struct RaycastingResult {
    /// Where did the ray hit the other shape?
    pub point: V2,
    /// If the ray didn't start inside the other shape, what is the normal?
    pub normal: Option<V2>,
    /// Did the ray start inside the shape?
    pub inside: bool,
}

pub(crate) use ray_aabb::*;
pub use tile_grid::*;
