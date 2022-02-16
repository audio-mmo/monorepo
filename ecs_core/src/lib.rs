//! Core types and traits for the ECS, which are used by the derive macros and networking code.
pub mod component;
pub mod identifiers;

pub use component::*;
pub use identifiers::*;
