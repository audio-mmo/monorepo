/// A menu.
///
/// This menu takes a set of items and some `T` to return, and then either resolves to the `T` or cancels.  The state of
/// the menu is polled with `poll_outcome`.
///
/// Internally, this is implemented as an atomic isize which can be in an unresolved or cancelled state using negative
/// numbers, and which otherwise represents an index into the vec of elements.
use std::sync::{
    atomic::{AtomicIsize, Ordering},
    Arc,
};
use uuid::Uuid;

use ammo_protos::frontend;

use crate::ui_elements::{UiElement, UiElementOperationResult};

const UNRESOLVED_STATE: isize = -2;
const CANCELLED_STATE: isize = -1;

pub struct SimpleMenu<T> {
    items: Vec<SimpleMenuItem<T>>,
    state: AtomicIsize,
    can_cancel: bool,
    title: String,
}

struct SimpleMenuItem<T> {
    label: String,
    // Sent to frontend and used to match things up at the end.
    key: uuid::Uuid,
    value: T,
}

pub enum SimpleMenuOutcome<'a, T> {
    /// We don't know yet.
    Unresolved,

    /// This menu resulted in selection of the specified item.
    Selected(&'a T),

    /// This menu was cancelled.
    Cancelled,
}

pub struct SimpleMenuBuilder<T> {
    title: String,
    can_cancel: bool,
    items: Vec<SimpleMenuItem<T>>,
}

impl<T> SimpleMenuBuilder<T> {
    pub fn new(title: String, can_cancel: bool) -> Self {
        SimpleMenuBuilder {
            title,
            items: vec![],
            can_cancel,
        }
    }

    pub fn add_item(&mut self, label: String, value: T) {
        self.items.push(SimpleMenuItem {
            label,
            value,
            key: Uuid::new_v4(),
        });
    }

    pub fn build(self) -> Arc<SimpleMenu<T>> {
        Arc::new(SimpleMenu {
            can_cancel: self.can_cancel,
            state: AtomicIsize::new(UNRESOLVED_STATE),
            items: self.items,
            title: self.title,
        })
    }
}

impl<T> SimpleMenu<T> {
    pub fn poll_outcome(&self) -> SimpleMenuOutcome<T> {
        let state = self.state.load(Ordering::Acquire);

        if state == CANCELLED_STATE {
            SimpleMenuOutcome::Cancelled
        } else if state == UNRESOLVED_STATE {
            SimpleMenuOutcome::Unresolved
        } else if state < 0 {
            panic!("Menu ended up in invalid state {}", state);
        } else {
            SimpleMenuOutcome::Selected(&self.items[state as usize].value)
        }
    }

    pub fn build_proto(&self) -> frontend::UiElement {
        let items = self
            .items
            .iter()
            .map(|i| {
                let key = format!("{:x}", i.key);
                frontend::MenuItem {
                    label: i.label.clone(),
                    key: key.clone(),
                    value: key,
                }
            })
            .collect();
        let menu = frontend::Menu {
            title: self.title.clone(),
            items,
        };
        frontend::UiElement {
            menu: Some(menu),
            ..Default::default()
        }
    }
}

impl<T: Send + Sync + 'static> UiElement for SimpleMenu<T> {
    fn get_initial_state(&self) -> anyhow::Result<frontend::UiElement> {
        Ok(self.build_proto())
    }

    fn do_complete(&self, value: String) -> anyhow::Result<super::UiElementOperationResult> {
        let uuid = Uuid::parse_str(&value)?;

        let mut ind: Option<usize> = None;
        for (i, x) in self.items.iter().enumerate() {
            if x.key == uuid {
                ind = Some(i);
            }
        }
        let ind = ind.ok_or_else(|| anyhow::anyhow!("Unable to find value in menu"))?;
        self.state.store(ind as isize, Ordering::Release);

        Ok(UiElementOperationResult::Finished)
    }

    fn do_cancel(&self) -> anyhow::Result<UiElementOperationResult> {
        if !self.can_cancel {
            anyhow::bail!("This menu may not be cancelled");
        }
        self.state.store(CANCELLED_STATE, Ordering::Release);

        Ok(UiElementOperationResult::Finished)
    }
}
