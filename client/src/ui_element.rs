//! The trait defining a UI element.
//!
//! UI elements are stored in a stack and polled for state updates every tick.  These state updates, if any, are sent to
//! the frontend.  Each element must return a proto representing itself.
//!
//! The trait is allowed to error, but this should be incredibly exceptional and it's only there so that we have a
//! second chance to catch errors without panicking across an FFI boundary.
use anyhow::Result;

use ammo_protos::frontend;

pub trait UiElement {
    /// Called every game tick, as well as at startup.
    ///
    /// The first tick is always communicated to the frontend.  Thereafter, only ticks which produce a different state
    /// are guaranteed to be sent over, but more ticks are possible, particularly if other UI elements are changing
    /// state.
    fn tick(&mut self) -> Result<frontend::UiElement>;

    /// This UI element was cancelled.
    ///
    /// This means, e.g., the user escaped out of a menu.
    fn do_cancel(&mut self) -> Result<()> {
        Ok(())
    }

    /// This UI element is "complete"
    ///
    /// This has a dedicated, element-specific meaning, but is for example the selected string from a  menu.
    fn do_complete(&mut self, _value: String) -> Result<()> {
        Ok(())
    }
}
