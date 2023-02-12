use crate::*;

pub(crate) mod ray_aabb;
pub mod tile_grid;

/// The result of performing a raycasting test.
#[derive(Debug)]
pub struct RaycastingResult {
    /// Where did the ray hit the other shape?
    pub point: V2<f64>,
    /// If the ray didn't start inside the other shape, what is the normal?
    pub normal: Option<V2<f64>>,
    /// Did the ray start inside the shape?
    pub inside: bool,
}

pub use tile_grid::*;
