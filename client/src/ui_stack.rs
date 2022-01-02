//! The stack of UI elements.
//!
//! This consists of a number of running trait elements and helper functions to apply actions.  The action functions as
//! well as the UI tick all return `Result<Option<frontend::UiStack>>`: `Ok(Some(x))` to send a new state to the
//! frontend, or `(Ok(None)` to skip.
//!
//! Actions are addressed to elements by key, not index.  Under the assumption that the frontend is going to pick up the
//! next state the next time it ticks, actions to non-existant elements are simply ignored.
//!
//! The stack is driven by a background thread and communicated with from the frontend via a `UiStackHandle`
//!
//! IMPORTANT: it is critical to iterate from the top of the stack to the bottom so that elements may remove themselves.
use std::sync::Arc;

use anyhow::Result;
use crossbeam::channel as chan;
use uuid::Uuid;

use ammo_protos::frontend;

use crate::ui_elements::{UiElement, UiElementOperationResult};

enum UiActionKind {
    Cancel,
    Complete(String),
}

struct UiAction {
    target: String,
    kind: UiActionKind,
}

struct UiStackHandleState {
    // The stack we last sent to the client, if any.
    stack: arc_swap::ArcSwapOption<frontend::UiStack>,
    action_sender: chan::Sender<UiAction>,
}

pub struct UiStack {
    /// The running UI elements.
    elements: Vec<Arc<dyn UiElement>>,

    /// The states of the UI elements.
    ///
    /// We need to maintain this separately, because the trait object erases what we need to know in order to do the
    /// bookkeeping.
    ///
    /// `None` means that this element hasn't yet been initialized.
    current_element_states: Vec<Option<frontend::UiStackEntry>>,

    handle_state: Arc<UiStackHandleState>,
    action_receiver: chan::Receiver<UiAction>,
}

pub struct UiStackHandle {
    state: Arc<UiStackHandleState>,
}

impl UiStack {
    pub fn new_with_handle() -> (UiStack, UiStackHandle) {
        let (action_sender, action_receiver) = chan::unbounded();

        let hs: Arc<UiStackHandleState> = Arc::new(UiStackHandleState {
            action_sender,
            stack: Default::default(),
        });
        let stack = UiStack {
            elements: Default::default(),
            current_element_states: Default::default(),
            handle_state: hs.clone(),
            action_receiver,
        };

        let handle = UiStackHandle { state: hs };
        (stack, handle)
    }

    fn remove_element(&mut self, index: usize) {
        self.elements.remove(index);
        self.current_element_states.remove(index);
    }

    /// Handle the result of a UI element call.
    ///
    /// Assumes the element was already initialized.
    fn handle_operation_result(
        &mut self,
        index: usize,
        outcome: UiElementOperationResult,
    ) -> Result<()> {
        match outcome {
            UiElementOperationResult::Finished => self.remove_element(index),
            UiElementOperationResult::ProposeState(s) => {
                self.current_element_states[index]
                    .as_mut()
                    .expect("Was already initialized")
                    .element = s
            }
            UiElementOperationResult::NothingChanged => {}
        }
        Ok(())
    }

    /// Initialize elements which need to be initialized.
    fn initialize_elements(&mut self) -> Result<()> {
        for i in (0..self.elements.len()).rev() {
            if self.current_element_states[i].is_none() {
                self.current_element_states[i] = Some(frontend::UiStackEntry {
                    element: self.elements[i].get_initial_state()?,
                    key: format!("{:x}", Uuid::new_v4()),
                });
            }
        }

        Ok(())
    }

    /// Tick the stack.
    pub fn tick(&mut self) -> Result<()> {
        self.initialize_elements()?;
        self.drain_actions()?;

        for i in (0..self.elements.len()).rev() {
            self.handle_operation_result(i, self.elements[i].tick()?)?;
        }

        // At the end of every tick, all elements should be initialized.  We don't want to support ui elements pushing
        // to the stack during their UI tick save perhaps through `UiElementOperationResult`.
        let mut stack: frontend::UiStack = Default::default();
        stack.entries.extend(
            self.current_element_states
                .iter()
                .map(|x| x.as_ref().expect("Should be initialized").clone()),
        );
        self.handle_state.stack.store(Some(Arc::new(stack)));
        Ok(())
    }

    fn find_element_by_key(&self, key: &str) -> Option<usize> {
        for (i, e) in self.current_element_states.iter().enumerate() {
            if let Some(ref x) = e {
                if x.key == key {
                    return Some(i);
                }
            }
        }

        None
    }

    fn drain_actions(&mut self) -> Result<()> {
        while let Ok(a) = self.action_receiver.try_recv() {
            if let Some(ind) = self.find_element_by_key(&a.target) {
                let e = &self.elements[ind];

                let outcome = match a.kind {
                    UiActionKind::Cancel => e.do_cancel()?,
                    UiActionKind::Complete(x) => e.do_complete(x)?,
                };

                self.handle_operation_result(ind, outcome)?;
            }
        }
        Ok(())
    }

    pub fn push_element(&mut self, element: Arc<dyn UiElement>) -> Result<()> {
        self.elements.push(element);
        self.current_element_states.push(None);
        Ok(())
    }
}

impl UiStackHandle {
    pub fn get_stack(&self) -> Option<Arc<frontend::UiStack>> {
        self.state.stack.load_full()
    }

    pub fn do_cancel(&self, target: String) -> Result<()> {
        self.state.action_sender.send(UiAction {
            target,
            kind: UiActionKind::Cancel,
        })?;
        Ok(())
    }

    pub fn do_complete(&self, target: String, value: String) -> Result<()> {
        self.state.action_sender.send(UiAction {
            target,
            kind: UiActionKind::Complete(value),
        })?;
        Ok(())
    }
}
