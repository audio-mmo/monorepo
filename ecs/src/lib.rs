extern crate ammo_ecs_derive;

pub mod component;
pub mod components;
pub mod frozen_map;
pub mod id_factory;
pub mod object_id;
pub mod prelude;
pub mod store;
pub mod store_map;
pub mod system;
pub mod worldlet;

pub use ammo_ecs_derive::*;

/// The reserved string namespace for ammo components.
pub const AMMO_NAMESPACE: &str = "ammo";
/// The reserved int namespace for ammo components.
pub const AMMO_INT_NAMESPACE: u16 = 1;
