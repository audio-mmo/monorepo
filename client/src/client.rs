use std::sync::Arc;

use anyhow::Result;

use ammo_protos::frontend;

use crate::main_thread::MainThreadHandle;

/// A running client.
///
/// This spawns a number of background threads, and integrates with a frontend.  Specifically:
///
/// - The frontend calls [Client::new] which initializes the client and kicks off background threads to run the
///   simulation and other such things.
/// - The frontend then repeatedly calls [Client::dequeue_service_requests] to get service requests such as speech and
///   shutdown, and [Client::get_ui_stack] to get updated UI stacks.
pub struct Client {
    main_thread: MainThreadHandle,
}

impl Client {
    pub fn new() -> Result<Self> {
        ammo_logging::log_to_stdout();

        Ok(Client {
            main_thread: crate::main_thread::spawn_main_thread()?,
        })
    }

    pub fn dequeue_service_requests(&self, dest: &mut Vec<frontend::ServiceRequest>) -> Result<()> {
        self.main_thread
            .frontend_service_provider()
            .extract_requests(dest)?;
        Ok(())
    }

    pub fn get_ui_stack(&self) -> Result<Option<Arc<frontend::UiStack>>> {
        Ok(self.main_thread.ui_stack().get_stack())
    }

    /// Send a request to a given UI element to complete with the specified value.
    pub fn do_complete(&self, target: String, value: String) -> Result<()> {
        self.main_thread.ui_stack().do_complete(target, value)
    }

    /// Instruct a specific UI element to cancel itself.
    pub fn do_cancel(&self, target: String) -> Result<()> {
        self.main_thread.ui_stack().do_cancel(target)
    }
}
