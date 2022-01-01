use std::sync::Arc;

use anyhow::Result;

use ammo_protos::frontend;

use crate::ui_stack::{UiStack, UiStackHandle};
use crate::world_state::WorldState;

pub struct MainThreadHandle {
    ui_stack_handle: UiStackHandle,
}

fn main_thread(mut ui_stack: UiStack, mut world_state: WorldState) {
    loop {
        ui_stack.tick(&mut world_state).expect("Should tick");
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

pub fn spawn_main_thread() -> Result<MainThreadHandle> {
    let (ui_stack, ui_stack_handle) = UiStack::new_with_handle();
    let world_state = WorldState::new();
    std::thread::spawn(move || main_thread(ui_stack, world_state));
    Ok(MainThreadHandle { ui_stack_handle })
}

impl MainThreadHandle {
    pub fn get_ui_stack(&self) -> Option<Arc<frontend::UiStack>> {
        self.ui_stack_handle.get_stack()
    }
}
