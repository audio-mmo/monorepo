use std::sync::Arc;

use anyhow::Result;
use crossbeam::channel as chan;
use log::*;
use synthizer as syz;

use crate::bootstrap::Bootstrap;
use crate::buffer::BufferHandle;
use crate::buffer_player::{BufferPlayer, BufferPlayerHandle};
use crate::command::{Command, CommandPayload};
use crate::decoding_pool::DecodingPool;
use crate::io_provider::IoProvider;
use crate::object::{Object, ObjectHandle};

/// Concurrency for the decoding pool.  Not currently exposed as a config value to users because we can effectively
/// always do the right thing: we know that this is I/O bound, not CPU bound, and needs to be a nice value to run on a
/// variety of machines.
const DECODING_CONCURRENCY: usize = 4;

/// How long is the decoding queue?
const DECODING_QUEUE_LENGTH: usize = 1024;

/// How many commands will we allow to be outstanding at once?
const COMMAND_QUEUE_LENGTH: usize = 1024;

pub(crate) struct EngineState {
    context: syz::Context,
    decoding_pool: DecodingPool,
    music_state: atomic_refcell::AtomicRefCell<Option<MusicState>>,
}

pub struct Engine {
    state: Arc<EngineState>,
    command_sender: chan::Sender<Command>,
}

pub(crate) struct MusicState {
    generator: syz::StreamingGenerator,
    source: syz::DirectSource,
}

fn engine_thread(context: syz::Context, cmd_receiver: chan::Receiver<Command>) {
    for c in cmd_receiver.iter() {
        c.execute(&context);
    }
}

impl MusicState {
    pub(crate) fn new(ctx: &syz::Context, stream: syz::StreamHandle) -> Result<MusicState> {
        let generator = syz::StreamingGenerator::from_stream_handle(ctx, stream)?;
        let source = syz::DirectSource::new(ctx)?;
        let linger_cfg = syz::DeleteBehaviorConfigBuilder::new()
            .linger(true)
            .linger_timeout(0.3)
            .build();
        generator.config_delete_behavior(&linger_cfg)?;
        source.config_delete_behavior(&linger_cfg)?;
        source.add_generator(&generator)?;
        Ok(MusicState { generator, source })
    }
}

impl EngineState {
    /// Always called from the background thread.  Configure music.
    pub(crate) fn set_music_bg(&self, key: &str) -> Result<()> {
        let sh = self.decoding_pool.get_stream_handle(key)?;
        let ms = MusicState::new(&self.context, sh)?;
        *self.music_state.borrow_mut() = Some(ms);
        Ok(())
    }

    pub(crate) fn clear_music_bg(&self) -> Result<()> {
        *self.music_state.borrow_mut() = None;
        Ok(())
    }
}

impl Engine {
    pub fn new(buffer_source: Box<dyn IoProvider>) -> Result<Arc<Engine>> {
        let decoding_pool =
            DecodingPool::new(DECODING_CONCURRENCY, DECODING_QUEUE_LENGTH, buffer_source)?;
        let (command_sender, command_receiver) = chan::bounded(COMMAND_QUEUE_LENGTH);
        let context = syz::Context::new()?;
        let bg_context = context.clone();
        context.orientation().set((0.0, 1.0, 0.0, 0.0, 0.0, 1.0))?;

        std::thread::spawn(move || engine_thread(bg_context, command_receiver));

        Ok(Arc::new(Engine {
            state: Arc::new(EngineState {
                context,
                decoding_pool,
                music_state: Default::default(),
            }),
            command_sender,
        }))
    }

    fn bootstrap_object(&self, what: Arc<dyn Bootstrap>) -> Result<()> {
        let cmd = Command::new(CommandPayload::Bootstrap(what), None);
        self.command_sender.send(cmd)?;
        Ok(())
    }

    /// Run a callback in the audio thread.
    #[allow(clippy::type_complexity)]
    pub(crate) fn run_callback(
        &self,
        callback: fn(
            Arc<dyn std::any::Any + Send + Sync>,
            (f64, f64, f64, f64, f64, f64),
        ) -> Result<()>,
        arg1: Arc<dyn std::any::Any + Send + Sync>,
        arg2: (f64, f64, f64, f64, f64, f64),
    ) -> Result<()> {
        let cp = CommandPayload::RunCallback {
            callback,
            arg1,
            arg2,
        };
        let cmd = Command::new(cp, None);
        self.command_sender.send(cmd)?;

        Ok(())
    }

    pub(crate) fn send_command(&self, payload: CommandPayload) -> Result<()> {
        let cmd = Command::new(payload, None);
        self.command_sender.send(cmd)?;
        Ok(())
    }

    /// Enqueue decoding for and return a handle to a buffer.
    pub fn new_buffer(self: &Arc<Self>, key: String) -> Result<BufferHandle> {
        debug!("Creation request for buffer using asset {}", key);
        Ok(BufferHandle(
            self.clone(),
            Arc::new(self.state.decoding_pool.decode(key.into())?),
        ))
    }

    pub fn new_object(
        self: &Arc<Self>,
        panner_strategy: syz::PannerStrategy,
        initial_pos: (f64, f64, f64),
    ) -> Result<ObjectHandle> {
        let obj = Arc::new(Object::new(panner_strategy, initial_pos)?);
        self.bootstrap_object(obj.clone())?;
        Ok(ObjectHandle(self.clone(), obj))
    }

    pub fn new_buffer_player(
        self: &Arc<Self>,
        buffer: &BufferHandle,
    ) -> Result<BufferPlayerHandle> {
        let bp = Arc::new(BufferPlayer::new(buffer.1.clone())?);
        self.bootstrap_object(bp.clone())?;
        Ok(BufferPlayerHandle(self.clone(), bp))
    }

    pub fn set_listener_position(&self, pos: (f64, f64, f64)) -> Result<()> {
        self.state.context.position().set(pos)?;
        Ok(())
    }

    pub fn set_listener_orientation(&self, at: (f64, f64, f64), up: (f64, f64, f64)) -> Result<()> {
        self.state
            .context
            .orientation()
            .set((at.0, at.1, at.2, up.0, up.1, up.2))?;
        Ok(())
    }

    /// Start a music track playing.
    ///
    /// Music can be stopped with `clear_music`.
    pub fn set_music(self: &Arc<Engine>, key: &str) -> Result<()> {
        let payload = CommandPayload::SetMusic(self.state.clone(), key.to_string());
        self.send_command(payload)
    }

    /// Clear/stop music.
    pub fn clear_music(self: &Arc<Self>) -> Result<()> {
        let payload = CommandPayload::ClearMusic(self.state.clone());
        self.send_command(payload)
    }
}
