use anyhow::Result;
use synthizer as syz;

/// encapsulates a Synthizer source, plus the position etc. of that source.
pub(crate) struct Object {
    source: syz::Source3D,
    position: (f64, f64, f64),
}

impl Object {
    pub(crate) fn new(
        ctx: &syz::Context,
        panner_strategy: syz::PannerStrategy,
        initial_pos: (f64, f64, f64),
    ) -> Result<Object> {
        let source = syz::Source3D::new(ctx, panner_strategy, initial_pos)?;
        Ok(Object {
            source,
            position: initial_pos,
        })
    }

    pub(crate) fn connect_to_object(&self, what: &dyn Connectable) -> Result<()> {
        what.connect(&self.source.clone().into())
    }

    pub(crate) fn disconnect_from_object(&self, what: &dyn Connectable) -> Result<()> {
        what.disconnect(&self.source.clone().into())
    }
}

/// Internal trait which encapsulates over everything that may connect to an object.
pub(crate) trait Connectable {
    fn connect(&self, src: &syz::Source) -> Result<()>;
    fn disconnect(&self, src: &syz::Source) -> Result<()>;
}
