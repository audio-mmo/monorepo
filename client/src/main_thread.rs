use std::sync::Arc;

use anyhow::Result;

use crate::frontend_service_provider::FrontendServiceProvider;
use crate::ui_stack::{UiStack, UiStackHandle};
use crate::world_state::WorldState;

pub struct MainThreadHandle {
    ui_stack_handle: UiStackHandle,
    frontend_service_provider: Arc<FrontendServiceProvider>,
}

fn main_thread(
    mut ui_stack: UiStack,
    _frontend_service_provider: Arc<FrontendServiceProvider>,
    _world_state: WorldState,
) {
    log::info!("Main thread starting up");

    loop {
        ui_stack.tick().expect("Should tick");
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

pub fn spawn_main_thread() -> Result<MainThreadHandle> {
    let (ui_stack, ui_stack_handle) = UiStack::new_with_handle();
    let world_state = WorldState::new();
    let frontend_service_provider = Arc::new(FrontendServiceProvider::new());
    let fsp_cloned = frontend_service_provider.clone();
    std::thread::spawn(move || main_thread(ui_stack, fsp_cloned, world_state));
    Ok(MainThreadHandle {
        ui_stack_handle,
        frontend_service_provider,
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
