use anyhow::Result;

use ammo_protos::frontend;

use crate::frontend_service_provider::FrontendServiceProvider;
use crate::ui_stack::UiStack;
use crate::world_state::WorldState;

/// A running client.
///
/// This spawns a number of background threads, and integrates with a frontend.  Specifically:
///
/// - The frontend calls [Client::new] which initializes the client and kicks off background threads to run the
///   simulation and other such things.
/// - The frontend then repeatedly calls [Client::dequeue_service_requests] to get service requests such as speech and
///   shutdown, and [Client::tick_ui] to get updated UI stacks.
pub struct Client {
    ui_stack: UiStack,
    frontend_service_provider: FrontendServiceProvider,
    world_state: WorldState,
}

impl Client {
    pub fn new() -> Result<Self> {
        Ok(Client {
            ui_stack: UiStack::new(),
            frontend_service_provider: FrontendServiceProvider::new(),
            world_state: WorldState::new(),
        })
    }

    pub fn dequeue_service_requests(&self, dest: &mut Vec<frontend::ServiceRequest>) -> Result<()> {
        self.frontend_service_provider.extract_requests(dest)?;
        Ok(())
    }

    pub fn tick_ui(&mut self) -> Result<Option<frontend::UiStack>> {
        self.ui_stack.tick(&mut self.world_state)
    }
}
