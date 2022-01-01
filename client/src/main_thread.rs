use anyhow::Result;

use crate::frontend_service_provider::FrontendServiceProvider;
use crate::ui_stack::{UiStack, UiStackHandle};
use crate::world_state::WorldState;

pub struct MainThreadHandle {
    ui_stack_handle: UiStackHandle,
    frontend_service_provider: FrontendServiceProvider,
}

fn main_thread(mut ui_stack: UiStack, _world_state: WorldState) {
    loop {
        ui_stack.tick().expect("Should tick");
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

pub fn spawn_main_thread() -> Result<MainThreadHandle> {
    let (ui_stack, ui_stack_handle) = UiStack::new_with_handle();
    let world_state = WorldState::new();
    std::thread::spawn(move || main_thread(ui_stack, world_state));
    Ok(MainThreadHandle {
        ui_stack_handle,
        frontend_service_provider: FrontendServiceProvider::new(),
    })
}

impl MainThreadHandle {
    pub fn ui_stack(&self) -> &UiStackHandle {
        &self.ui_stack_handle
    }

    pub fn frontend_service_provider(&self) -> &FrontendServiceProvider {
        &self.frontend_service_provider
    }
}
