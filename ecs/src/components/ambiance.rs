use crate::prelude::*;

/// The integral id for the [Ambiance] component.
pub const AMBIANCE_ID: u16 = 3;
/// The string id for the [Ambiance] component.
pub const AMBIANCE_ID_STR: &str = "ambiance";

/// Attach a sound to an object.
#[derive(Clone, Component, Debug, serde::Deserialize, serde::Serialize)]
#[ammo(
    namespace = "AMMO_NAMESPACE",
    id = "AMBIANCE_ID_STR",
    int_namespace = "AMMO_INT_NAMESPACE",
    int_id = "AMBIANCE_ID"
)]
#[serde(tag = "type", content = "payload")]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Ambiance {
    ConstantlyLoopingSound { asset_name: String },
}
