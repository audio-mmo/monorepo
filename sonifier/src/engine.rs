use std::any::Any;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossbeam::{channel as chan, select};
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

/// How often do we wake up for non-realtime timeouts, etc?
const MAINTENANCE_TICK_INTERVAL: Duration = Duration::from_millis(100);
const MAINTENANCE_TICK_JITTER: Duration = Duration::from_millis(50);

pub(crate) struct EngineState {
    pub(crate) context: syz::Context,
    pub(crate) decoding_pool: Arc<DecodingPool>,
    pub(crate) music_state: Option<MusicState>,
    command_receiver: chan::Receiver<Command>,
    /// We use this channel closing as a signal to the background thread to terminate.
    shutdown_receiver: chan::Receiver<()>,
    /// This vector contains things that we want to hold onto until a specific duration has elapsed.
    ///
    /// We use Durations as the keys because in future we're going to want to be able to pause the countdown, so just
    /// using `Instant` isn't sufficient.  When the duration goes to 0, we delete the object.
    retain_until: Vec<(Duration, Arc<dyn Any>)>,
}

pub struct Engine {
    context: syz::Context,
    command_sender: chan::Sender<Command>,
    shutdown_sender: chan::Sender<()>,
    decoding_pool: Arc<DecodingPool>,
}

pub(crate) struct MusicState {
    generator: syz::StreamingGenerator,
    source: syz::DirectSource,
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
    pub(crate) fn set_music_bg(&mut self, key: &str) -> Result<()> {
        let sh = self.decoding_pool.get_stream_handle(key)?;
        let ms = MusicState::new(&self.context, sh)?;
        self.music_state = Some(ms);
        Ok(())
    }

    pub(crate) fn clear_music_bg(&mut self) -> Result<()> {
        self.music_state = None;
        Ok(())
    }

    fn maintenance_retain_until(&mut self, since_last: Duration) {
        // Retain doesn't give us mutable access; do the subtractions first.
        for (ref mut k, _) in self.retain_until.iter_mut() {
            *k = k.saturating_sub(since_last);
        }

        self.retain_until.retain(|(k, _)| *k != Duration::ZERO);
    }

    /// Run maintenance.
    ///
    /// Takes the time since the last maintenance tick.
    fn run_maintenance(&mut self, since_last: Duration) {
        self.maintenance_retain_until(since_last);
    }

    /// Hang onto an Arc until at least a given duration has elapsed.
    pub fn retain_until<T: Any>(&mut self, what: Arc<T>, duration: Duration) {
        // Introduce some slop here, so that if maintenance ticks happen early etc. we don't release too soon.
        self.retain_until
            .push((duration + MAINTENANCE_TICK_INTERVAL * 5, what));
    }
}

fn simple_jitter(dur: Duration, jitter: Duration) -> Duration {
    use rand::prelude::*;

    let scale = thread_rng().gen_range(0.0f64..=1.0);
    dur + Duration::from_secs_f64(jitter.as_secs_f64() * scale)
}

fn engine_thread(mut state: EngineState) {
    let mut last_maintenance_time = Instant::now();

    loop {
        let maintenance_receiver = chan::after(simple_jitter(
            MAINTENANCE_TICK_INTERVAL,
            MAINTENANCE_TICK_JITTER,
        ));

        select! {
            recv(state.command_receiver)-> r => if let Ok(command) = r {
                command.execute(&mut state);
            },
            recv(maintenance_receiver)-> _ => {
                let now = Instant::now();
                state.run_maintenance(now -last_maintenance_time);
                last_maintenance_time = now;
            },
            recv(state.shutdown_receiver) -> _ => {
                info!("Engine thread shutting down");
                break;
            }
        }
    }
}

impl Engine {
    pub fn new(buffer_source: Box<dyn IoProvider>) -> Result<Arc<Engine>> {
        let decoding_pool = Arc::new(DecodingPool::new(
            DECODING_CONCURRENCY,
            DECODING_QUEUE_LENGTH,
            buffer_source,
        )?);
        let (command_sender, command_receiver) = chan::bounded(COMMAND_QUEUE_LENGTH);
        let (shutdown_sender, shutdown_receiver) = chan::bounded(0);
        let context = syz::Context::new()?;
        context.orientation().set((0.0, 1.0, 0.0, 0.0, 0.0, 1.0))?;

        let bg_context = context.clone();
        let bg_pool = decoding_pool.clone();
        std::thread::spawn(move || {
            engine_thread(EngineState {
                context: bg_context,
                decoding_pool: bg_pool,
                music_state: Default::default(),
                command_receiver,
                shutdown_receiver,
                retain_until: Vec::with_capacity(256),
            })
        });

        Ok(Arc::new(Engine {
            context,
            decoding_pool,
            command_sender,
            shutdown_sender,
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
            Arc::new(self.decoding_pool.decode(key.into())?),
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
        self.context.position().set(pos)?;
        Ok(())
    }

    pub fn set_listener_orientation(&self, at: (f64, f64, f64), up: (f64, f64, f64)) -> Result<()> {
        self.context
            .orientation()
            .set((at.0, at.1, at.2, up.0, up.1, up.2))?;
        Ok(())
    }

    /// Start a music track playing.
    ///
    /// Music can be stopped with `clear_music`.
    pub fn set_music(self: &Arc<Engine>, key: &str) -> Result<()> {
        let payload = CommandPayload::SetMusic(key.to_string());
        self.send_command(payload)
    }

    /// Clear/stop music.
    pub fn clear_music(self: &Arc<Self>) -> Result<()> {
        let payload = CommandPayload::ClearMusic();
        self.send_command(payload)
    }
}
