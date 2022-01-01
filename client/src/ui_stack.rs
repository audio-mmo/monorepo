//! The stack of UI elements.
//!
//! This consists of a number of running trait elements and helper functions to apply actions.  The action functions as
//! well as the UI tick all return `Result<Option<frontend::UiStack>>`: `Ok(Some(x))` to send a new state to the
//! frontend, or `(Ok(None)` to skip.
//!
//! Actions are addressed to elements by key, not index.  Under the assumption that the frontend is going to pick up the
//! next state the next time it ticks, actions to non-existant elements are simply ignored.
use anyhow::Result;

use ammo_protos::frontend;

use crate::ui_elements::{UiElement, UiElementDef, UiElementOperationResult};
use crate::world_state::WorldState;

#[derive(Default)]
pub struct UiStack {
    /// The running UI elements.
    elements: Vec<Box<dyn UiElement>>,

    /// The states of the UI elements.
    ///
    /// We need to maintain this separately, because the trait object erases what we need to know in order to do the
    /// bookkeeping.
    ///
    /// `None` means that this element hasn't yet been initialized.
    current_element_states: Vec<Option<frontend::UiStackEntry>>,

    /// The stack we last sent to the client.
    last_sent: frontend::UiStack,
}

impl UiStack {
    pub fn new() -> UiStack {
        Default::default()
    }

    /// Tick the stack.
    pub fn tick(&mut self, world_state: &mut WorldState) -> Result<Option<frontend::UiStack>> {
        // We would really like to use retain, but that doesn't give us a mutable reference.  Instead, iterate in
        // reverse order using a range, popping elements as we go if needed
        for i in (0..self.elements.len()).rev() {
            use UiElementOperationResult::*;

            let def = UiElementDef {
                world_state,
                stack_index: i,
            };

            if self.current_element_states[i].is_none() {
                self.current_element_states[i] = Some(frontend::UiStackEntry {
                    element: self.elements[i].get_initial_state(&def)?,
                    key: "foo".into(),
                });
            }

            match self.elements[i].tick(&def)? {
                NothingChanged => continue,
                Finished => {
                    self.current_element_states.remove(i);
                    self.elements.remove(i);
                }
                ProposeState(s) => {
                    self.current_element_states[i]
                        .as_mut()
                        .expect("Should have been initialized already")
                        .element = s;
                }
            }
        }

        // At the end of every tick, all elements should be initialized.  We don't want to support ui elements pushing
        // to the stack during their UI tick save perhaps through `UiElementOperationResult`.
        let mut stack: frontend::UiStack = Default::default();
        stack.entries.extend(
            self.current_element_states
                .iter()
                .map(|x| x.as_ref().expect("Should be initialized").clone()),
        );
        self.last_sent = stack.clone();
        Ok(Some(stack))
    }
}
