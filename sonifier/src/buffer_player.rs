use std::sync::Arc;

use anyhow::Result;
use synthizer as syz;

use crate::buffer::Buffer;
use crate::object::ConnectionSupport;

/// Plays buffers and offers basic controls.
///
/// This is the boring one: just a Synthizer generator.
pub(crate) struct BufferPlayer {
    buffer: Arc<syz::Buffer>,
    generator: syz::BufferGenerator,
}

impl BufferPlayer {
    pub(crate) fn new(ctx: &syz::Context, mut buffer: Buffer) -> Result<BufferPlayer> {
        let gen = syz::BufferGenerator::new(ctx)?;
        let sbuf = buffer.as_synthizer()?;
        gen.buffer().set(&*sbuf)?;
        Ok(BufferPlayer {
            buffer: sbuf,
            generator: gen,
        })
    }

    pub(crate) fn pause(&self) -> Result<()> {
        self.generator.pause()?;
        Ok(())
    }

    pub(crate) fn play(&self) -> Result<()> {
        self.generator.play()?;
        Ok(())
    }
}

impl ConnectionSupport for BufferPlayer {
    fn connect(&self, src: &syz::Source) -> Result<()> {
        src.add_generator(&self.generator)?;
        Ok(())
    }

    fn disconnect(&self, src: &syz::Source) -> Result<()> {
        src.remove_generator(&self.generator)?;
        Ok(())
    }
}
