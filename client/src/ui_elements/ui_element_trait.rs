//! The trait defining a UI element.
//!
//! UI elements are stored in a stack and polled for state updates every tick.  These state updates, if any, are sent to
//! the frontend.  Each element must return a proto representing itself.
//!
//! The trait is allowed to error, but this should be incredibly exceptional and it's only there so that we have a
//! second chance to catch errors without panicking across an FFI boundary.
use anyhow::Result;

use ammo_protos::frontend;

use crate::world_state::WorldState;

/// Info needed to uniquely identify a UI element, and the state that it goes with.
///
/// Elements may, e.g., shift position in the stack. Every element in the below trait thus receives an immutable
/// reference to this struct, so that the trait can know the global state.
/// and similar.
pub struct UiElementDef<'a> {
    pub stack_index: usize,
    pub world_state: &'a mut WorldState,
}

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

pub trait UiElement: Send {
    /// Called exactly once after the element is in the stack.  Must return the initial state.
    fn get_initial_state(&mut self, ui_def: &UiElementDef) -> Result<frontend::UiElement>;

    /// Called every game tick, as well as at startup.
    ///
    /// Only ticks which produce a different state are guaranteed to be sent over, but more ticks are possible,
    /// particularly if other UI elements are changing state.
    ///
    /// The default implementation never updates the state.  This is the most common case.
    fn tick(&mut self, _ui_def: &UiElementDef) -> Result<UiElementOperationResult> {
        Ok(UiElementOperationResult::NothingChanged)
    }

    /// This UI element was cancelled.
    ///
    /// This means, e.g., the user escaped out of a menu.
    ///
    /// The default action is to close the element.
    fn do_cancel(&mut self, _ui_def: &UiElementDef) -> Result<UiElementOperationResult> {
        Ok(UiElementOperationResult::Finished)
    }

    /// This UI element is "complete"
    ///
    /// This has a dedicated, element-specific meaning, but is for example the selected string from a  menu.
    ///
    /// The default implementation pops the element from the stack without doing anything.
    fn do_complete(
        &mut self,
        _ui_state: &UiElementDef,
        _value: String,
    ) -> Result<UiElementOperationResult> {
        Ok(UiElementOperationResult::Finished)
    }
}
