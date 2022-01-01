/// A menu.
///
/// This menu takes a set of items and some `T` to return, and then either resolves to the `T` or cancels.  The state of
/// the menu is polled with `poll_outcome`.
use uuid::Uuid;

use ammo_protos::frontend;

use crate::ui_elements::{UiElement, UiElementOperationResult};

pub struct SimpleMenu<T> {
    items: Vec<SimpleMenuItem<T>>,
    state: SimpleMenuState,
    can_cancel: bool,
    title: String,
}

struct SimpleMenuItem<T> {
    label: String,
    // Sent to frontend and used to match things up at the end.
    key: uuid::Uuid,
    value: T,
}

enum SimpleMenuState {
    Unresolved,
    Selected(usize),
    Cancelled,
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

    pub fn build(self) -> SimpleMenu<T> {
        SimpleMenu {
            can_cancel: self.can_cancel,
            state: SimpleMenuState::Unresolved,
            items: self.items,
            title: self.title,
        }
    }
}

impl<T> SimpleMenu<T> {
    pub fn poll_outcome(&mut self) -> SimpleMenuOutcome<T> {
        match &self.state {
            SimpleMenuState::Unresolved { .. } => SimpleMenuOutcome::Unresolved,
            SimpleMenuState::Selected(x) => SimpleMenuOutcome::Selected(&self.items[*x].value),
            SimpleMenuState::Cancelled => SimpleMenuOutcome::Cancelled,
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

impl<T> UiElement for SimpleMenu<T> {
    fn get_initial_state(
        &mut self,
        _ui_def: &super::UiElementDef,
    ) -> anyhow::Result<frontend::UiElement> {
        Ok(self.build_proto())
    }

    fn do_complete(
        &mut self,
        _ui_state: &super::UiElementDef,
        value: String,
    ) -> anyhow::Result<super::UiElementOperationResult> {
        let uuid = Uuid::parse_str(&value)?;

        let mut ind: Option<usize> = None;
        for (i, x) in self.items.iter().enumerate() {
            if x.key == uuid {
                ind = Some(i);
            }
        }
        let ind = ind.ok_or_else(|| anyhow::anyhow!("Unable to find value in menu"))?;
        self.state = SimpleMenuState::Selected(ind);

        Ok(UiElementOperationResult::Finished)
    }

    fn do_cancel(
        &mut self,
        _ui_def: &super::UiElementDef,
    ) -> anyhow::Result<UiElementOperationResult> {
        if !self.can_cancel {
            anyhow::bail!("This menu may not be cancelled");
        }
        self.state = SimpleMenuState::Cancelled;

        Ok(UiElementOperationResult::Finished)
    }
}
