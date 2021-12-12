use std::any::Any;
use std::sync::Arc;

use anyhow::Result;
use crossbeam::channel as chan;
use log::*;

use crate::bootstrap::Bootstrap;
use crate::engine::EngineState;
use crate::object::{Connectable, Object};

/// Commands that may be sent to the engine's processing thread.
pub(crate) enum CommandPayload {
    Bootstrap(Arc<dyn Bootstrap>),
    Connect(Arc<dyn Connectable>, Arc<Object>),
    Disconnect(Arc<dyn Connectable>, Arc<Object>),
    /// Run callbacks in the audio thread, avoiding the overhead of boxed closures or the need to infinitely expand this
    /// enum.
    RunCallback {
        #[allow(clippy::type_complexity)]
        callback: fn(Arc<dyn Any + Sync + Send>, (f64, f64, f64, f64, f64, f64)) -> Result<()>,
        arg1: Arc<dyn Any + Sync + Send>,
        arg2: (f64, f64, f64, f64, f64, f64),
    },
    SetMusic(String),
    ClearMusic(),
}

pub(crate) struct Command {
    payload: CommandPayload,
    /// If set, send the result back over this channel to the caller.
    result_channel: Option<chan::Sender<Result<()>>>,
}

impl CommandPayload {
    fn execute(self, state: &EngineState) -> Result<()> {
        match self {
            CommandPayload::Bootstrap(x) => x.bootstrap(&state.context),
            CommandPayload::Connect(src, dest) => dest.connect_to_object(&*src),
            CommandPayload::Disconnect(src, dest) => dest.disconnect_from_object(&*src),
            CommandPayload::RunCallback {
                callback,
                arg1,
                arg2,
            } => callback(arg1, arg2),
            CommandPayload::SetMusic(key) => state.set_music_bg(&key),
            CommandPayload::ClearMusic() => state.clear_music_bg(),
        }
    }
}

impl Command {
    pub(crate) fn new(
        payload: CommandPayload,
        result_channel: Option<chan::Sender<Result<()>>>,
    ) -> Command {
        Command {
            payload,
            result_channel,
        }
    }

    pub(crate) fn execute(self, state: &EngineState) {
        let res = self.payload.execute(state).map_err(|e| {
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
