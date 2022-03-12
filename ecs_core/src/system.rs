//! Defines the [System] trait.
//!
//! A system is a simulation step.  It gets access to a worldlet.  The typical execution flow is:
//!
//! - Grab the stores you need off the worldlet, once at the beginning.
//! - Do something with them.
//!
//! Systems run one worldlet at a time.  There is intentionally no global context at this level of the hierarchy: a
//! worldlet is the entire universe from the perspective of a system, save if the system should arrange for other
//! communication channels (e.g. a global resource).
use std::any::Any;

use anyhow::Result;

use crate::store_map::StoreMap;
use crate::system_map::SystemMap;
use crate::worldlet::Worldlet;

/// The system trait. See module-level documentation.
pub trait System: Any + Send + Sync + Default {
    /// Run the system
    ///
    /// Systems can be fallible.  If this is the case, it should only fail if the world would be left in an inconsistent
    /// state; this is an escape hatch to do something better than a panic (for example scream at alerting), not a
    /// general-purpose mechanism.  Systems should try their best to be pure.
    fn execute<StoreM: StoreMap, SysM: SystemMap>(
        &mut self,
        worldlet: &Worldlet<StoreM, SysM>,
    ) -> Result<()>;
}
