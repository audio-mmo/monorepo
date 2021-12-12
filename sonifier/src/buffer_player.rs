use std::sync::Arc;

use anyhow::Result;
use synthizer as syz;

use crate::bootstrap::Bootstrap;
use crate::buffer::Buffer;
use crate::command::CommandPayload;
use crate::object::{Connectable, ObjectHandle};
use crate::Engine;

/// Plays buffers and offers basic controls.
///
/// This is the boring one: just a Synthizer generator.
pub(crate) struct BufferPlayer {
    buffer: Arc<Buffer>,
    /// Set after bootstrap.
    generator: atomic_refcell::AtomicRefCell<Option<syz::BufferGenerator>>,
}

impl BufferPlayer {
    pub(crate) fn new(buffer: Arc<Buffer>) -> Result<BufferPlayer> {
        Ok(BufferPlayer {
            buffer,
            generator: atomic_refcell::AtomicRefCell::new(None),
        })
    }

    pub(crate) fn pause(&self) -> Result<()> {
        self.generator.borrow().as_ref().unwrap().pause()?;
        Ok(())
    }

    pub(crate) fn play(&self) -> Result<()> {
        self.generator.borrow().as_ref().unwrap().play()?;
        Ok(())
    }

    pub(crate) fn set_looping(&self, looping: bool) -> Result<()> {
        self.generator
            .borrow()
            .as_ref()
            .unwrap()
            .looping()
            .set(looping)?;
        Ok(())
    }
}

impl Connectable for BufferPlayer {
    fn connect(&self, src: &syz::Source) -> Result<()> {
        src.add_generator(self.generator.borrow().as_ref().unwrap())?;
        Ok(())
    }

    fn disconnect(&self, src: &syz::Source) -> Result<()> {
        src.remove_generator(self.generator.borrow().as_ref().unwrap())?;
        Ok(())
    }
}

impl Bootstrap for BufferPlayer {
    fn bootstrap(&self, ctx: &syz::Context) -> Result<()> {
        let gen = syz::BufferGenerator::new(ctx)?;
        let sbuf = self.buffer.as_synthizer()?;
        gen.buffer().set(&*sbuf)?;
        gen.config_delete_behavior(&syz::DeleteBehaviorConfigBuilder::new().linger(true).build())?;
        *self.generator.borrow_mut() = Some(gen);

        Ok(())
    }
}

/// A reference-counted handle to a buffer player.
///
/// Buffer players know how to play buffers with optional looping, without any extra fancy logic on top of that.  These
/// are intended for short snippets of audio, and will configure Synthizer to linger: when the last reference is
/// dropped, the buffer will stop looping and finish the final loop iteration.
#[derive(Clone)]
pub struct BufferPlayerHandle(pub(crate) Arc<Engine>, pub(crate) Arc<BufferPlayer>);

impl BufferPlayerHandle {
    pub fn play(&self) -> Result<()> {
        self.0.run_callback(
            |x| x.downcast::<BufferPlayer>().unwrap().pause(),
            self.1.clone(),
        )
    }

    pub fn pause(&self) -> Result<()> {
        self.0.run_callback(
            |x| x.downcast::<BufferPlayer>().unwrap().play(),
            self.1.clone(),
        )
    }

    pub fn connect(&self, obj: &ObjectHandle) -> Result<()> {
        let cmd = CommandPayload::Connect(self.1.clone(), obj.1.clone());
        self.0.send_command(cmd)
    }

    pub fn disconnect(&self, obj: &ObjectHandle) -> Result<()> {
        let cmd = CommandPayload::Disconnect(self.1.clone(), obj.1.clone());
        self.0.send_command(cmd)
    }

    pub fn set_looping(&self, looping: bool) -> Result<()> {
        if looping {
            self.0.run_callback(
                |x| x.downcast::<BufferPlayer>().unwrap().set_looping(true),
                self.1.clone(),
            )?;
        } else {
            self.0.run_callback(
                |x| x.downcast::<BufferPlayer>().unwrap().set_looping(false),
                self.1.clone(),
            )?;
        }

        Ok(())
    }
}
