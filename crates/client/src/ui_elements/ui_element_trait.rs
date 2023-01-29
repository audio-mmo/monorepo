//! The trait defining a UI element.
//!
//! UI elements are stored in a stack and polled for state updates every tick.  These state updates, if any, are sent to
//! the frontend.  Each element must return a proto representing itself.
//!
//! The trait is allowed to error, but this should be incredibly exceptional and it's only there so that we have a
//! second chance to catch errors without panicking across an FFI boundary.
//!
//! Ui elemernts generally implement their functionality using interior mutability, and are pushed onto the stack using
//! Arc (and thus must be sync).  Despite the sync requirement, the UI elements will only have their [UiElement]
//! implementations called from one thread, so using things like `atomic_refcell` may be appropriate.
use anyhow::Result;

use ammo_protos::frontend;

/// Results of a UI element operation.
///
/// A UI element in itself is only reactive, and can simply respond to ticks.  This enum lets a UI element close itself,
/// among other things.
pub enum UiElementOperationResult {
    /// Nothing changed due to this operation.
    NothingChanged,
    /// Possibly send this state to the frontend, if the state actually changed.
    ProposeState(frontend::UiElement),
    /// This UI element is finished and should be removed from the stack.
    ///
    /// This isn't the only way for UI elements to be removed.  Other non-UI components control the reactive UI and may
    /// opt to remove the element outside the element's control.
    Finished,
}

pub trait UiElement: Send + Sync + 'static {
    /// Called exactly once after the element is in the stack.  Must return the initial state.
    fn get_initial_state(&self) -> Result<frontend::UiElement>;

    /// Called every game tick, as well as at startup.
    ///
    /// Only ticks which produce a different state are guaranteed to be sent over, but more ticks are possible,
    /// particularly if other UI elements are changing state.
    ///
    /// The default implementation never updates the state.  This is the most common case.
    fn tick(&self) -> Result<UiElementOperationResult> {
        Ok(UiElementOperationResult::NothingChanged)
    }

    /// This UI element was cancelled.
    ///
    /// This means, e.g., the user escaped out of a menu.
    ///
    /// The default action is to close the element.
    fn do_cancel(&self) -> Result<UiElementOperationResult> {
        Ok(UiElementOperationResult::Finished)
    }

    /// This UI element is "complete"
    ///
    /// This has a dedicated, element-specific meaning, but is for example the selected string from a  menu.
    ///
    /// The default implementation pops the element from the stack without doing anything.
    fn do_complete(&self, _value: String) -> Result<UiElementOperationResult> {
        Ok(UiElementOperationResult::Finished)
    }
}
