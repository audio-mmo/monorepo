use ammo_physics::V2;

use crate::prelude::*;

/// The id of the [Position] component.
pub const POSITION_ID: u16 = 1;
/// The string id of the [Position] component.
pub const POSITION_ID_STR: &str = "position";

/// Represents a position.
#[derive(Component, Clone, Debug, serde::Serialize, serde::Deserialize)]
#[ammo(
    namespace = "AMMO_NAMESPACE",
    id = "POSITION_ID_STR",
    int_namespace = "AMMO_INT_NAMESPACE",
    int_id = "POSITION_ID"
)]
pub struct Position {
    x: f64,
    y: f64,
}

impl Position {
    pub fn new(x: f64, y: f64) -> Position {
        Position { x, y }
    }

    pub fn as_v2(&self) -> V2 {
        V2::new(self.x, self.y)
    }
}
