pub use crate::Component;
pub use crate::{AMMO_INT_NAMESPACE, AMMO_NAMESPACE};
pub use ammo_ecs_core::{Component, ComponentExt, IntComponentId, StringComponentId};

// Reexport for the derive macros.  This is needed because we want to be able to use the derive macros in this crate, as
// well as in other crates.  In this crate, `ammo_ecs::` doesn't work.
//
// It's probably possible to do this better somehow, as it's now required that users import this prelude; we'll have to
// figure that out once we have more code so that we can get good coverage on what might or might not break the macro.
pub use crate as ammo_ecs_macro_reexport;
