use ammo_ecs_core::prelude::*;

/// The id of the [Shape] component.
const SHAPE_ID: u16 = 2;
/// The string id of the [Shape] component.
const SHAPE_ID_STR: &str = "shape";

#[derive(
    Clone,
    ammo_ecs_derive::Component,
    ammo_ecs_derive::HasIdentifiers,
    Debug,
    serde::Deserialize,
    serde::Serialize,
)]
#[ammo_idents(
    namespace = "AMMO_STRING_NAMESPACE",
    id = "SHAPE_ID_STR",
    int_namespace = "AMMO_INT_NAMESPACE",
    int_id = "SHAPE_ID"
)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type", content = "payload")]
#[non_exhaustive]
pub enum Shape {
    /// The object is a circle.
    Circle { radius: f64 },
    /// The object is a box.
    Box { width: f64, height: f64 },
}
