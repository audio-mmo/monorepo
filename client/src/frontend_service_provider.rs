//! Provides the code to let the client call into the frontend.
use anyhow::Result;
use crossbeam::channel as chan;

use ammo_protos::frontend::{self, ServiceRequest};

pub struct FrontendServiceProvider {
    request_sender: chan::Sender<ServiceRequest>,
    request_receiver: chan::Receiver<ServiceRequest>,
}

impl FrontendServiceProvider {
    pub fn new() -> Self {
        let (request_sender, request_receiver) = chan::unbounded();
        Self {
            request_sender,
            request_receiver,
        }
    }

    pub fn speak(&self, text: &str, interrupt: bool) -> Result<()> {
        let command_payload = frontend::SpeakRequest {
            interrupt,
            text: text.to_string(),
        };
        let req: frontend::ServiceRequest = frontend::ServiceRequest {
            service: Some(frontend::service_request::Service::Speech(command_payload)),
        };
        self.request_sender.send(req)?;
        Ok(())
    }

    pub fn shutdown(&self) -> Result<()> {
        let cmd: ServiceRequest = ServiceRequest {
            service: Some(frontend::service_request::Service::Shutdown(
                Default::default(),
            )),
        };
        self.request_sender.send(cmd)?;
        Ok(())
    }

    /// Extracct all of the pending commands.
    pub fn extract_requests(&self, dest: &mut Vec<ServiceRequest>) -> Result<()> {
        while let Ok(r) = self.request_receiver.try_recv() {
            dest.push(r);
        }

        Ok(())
    }
}

impl Default for FrontendServiceProvider {
    fn default() -> Self {
        Self::new()
    }
}
