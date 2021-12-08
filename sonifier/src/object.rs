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
}
