use std::sync::Arc;

use anyhow::Result;
use synthizer as syz;

use crate::bootstrap::Bootstrap;
use crate::Engine;

/// encapsulates a Synthizer source, plus the position etc. of that source.
pub(crate) struct Object {
    initial_pos: (f64, f64, f64),
    panner_strategy: syz::PannerStrategy,
    source: atomic_refcell::AtomicRefCell<Option<syz::Source3D>>,
    position: (f64, f64, f64),
}

impl Object {
    pub(crate) fn new(
        panner_strategy: syz::PannerStrategy,
        initial_pos: (f64, f64, f64),
    ) -> Result<Object> {
        Ok(Object {
            initial_pos,
            panner_strategy,
            source: atomic_refcell::AtomicRefCell::new(None),
            position: initial_pos,
        })
    }

    fn get_source(&self) -> atomic_refcell::AtomicRef<syz::Source3D> {
        atomic_refcell::AtomicRef::map(self.source.borrow(), |x| x.as_ref().unwrap())
    }

    pub(crate) fn connect_to_object(&self, what: &dyn Connectable) -> Result<()> {
        what.connect(&(*self.get_source()).clone().into())
    }

    pub(crate) fn disconnect_from_object(&self, what: &dyn Connectable) -> Result<()> {
        what.disconnect(&(*self.get_source()).clone().into())
    }
}

/// Internal trait which encapsulates over everything that may connect to an object.
pub(crate) trait Connectable: Send + Sync {
    fn connect(&self, src: &syz::Source) -> Result<()>;
    fn disconnect(&self, src: &syz::Source) -> Result<()>;
}

impl Bootstrap for Object {
    fn bootstrap(&self, ctx: &syz::Context) -> Result<()> {
        let source = syz::Source3D::new(ctx, self.panner_strategy, self.initial_pos)?;
        *self.source.borrow_mut() = Some(source);
        Ok(())
    }
}

/// A reference-counted handle to an audio object.
#[derive(Clone)]
pub struct ObjectHandle(pub(crate) Arc<Engine>, pub(crate) Arc<Object>);
