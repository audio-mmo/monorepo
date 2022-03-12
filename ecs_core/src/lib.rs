pub mod component;
pub mod id_factory;
pub mod identifiers;
pub mod object_id;
pub mod prelude;
pub mod store;
pub mod store_map;
pub mod system;
pub mod system_map;
pub mod time;
pub mod version;
pub mod worldlet;

pub use component::*;
pub use identifiers::*;

pub const AMMO_INT_NAMESPACE: u16 = 1;
pub const AMMO_STRING_NAMESPACE: &str = "ammo";

