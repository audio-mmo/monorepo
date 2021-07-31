use crate::*;

pub(crate) mod ray_circle;
pub mod tile_grid;

#[derive(Debug)]
pub(crate) struct RaycastingResult {
    /// Where did the ray hit the other shape?
    pub(crate) point: V2,
    /// If the ray didn't start inside the other shape, what is the normal?
    pub(crate) normal: Option<V2>,
    /// Did the ray start inside the shape?
    pub(crate) inside: bool,
}

pub(crate) use ray_circle::*;
pub use tile_grid::*;
