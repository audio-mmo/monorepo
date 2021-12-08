use std::sync::Arc;

use anyhow::Result;
use crossbeam::channel as chan;
use log::*;

use crate::object::{Connectable, Object};

/// Commands that may be sent to the engine's processing thread.
enum CommandPayload {
    Connect(Arc<dyn Connectable>, Arc<Object>),
    Disconnect(Arc<dyn Connectable>, Arc<Object>),
}

struct Command {
    payload: CommandPayload,
    /// If set, send the result back over this channel to the caller.
    result_channel: Option<chan::Sender<Result<()>>>,
}

impl CommandPayload {
    fn execute(&self) -> Result<()> {
        match self {
            CommandPayload::Connect(src, dest) => dest.connect_to_object(&**src),
            CommandPayload::Disconnect(src, dest) => dest.disconnect_from_object(&**src),
        }
    }
}

impl Command {
    pub(crate) fn execute(&self) {
        let res = self.payload.execute().map_err(|e| {
            error!("Error executing audio command: {:?}", e);
            e
        });

        if let Some(c) = self.result_channel.as_ref() {
            if let Err(e) = c.send(res) {
                debug!("Could not send over rsult channel: {:?}", e);
            }
        }
    }
}
