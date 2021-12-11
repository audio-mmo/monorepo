use anyhow::Result;
use synthizer as syz;

/// Handles the need of an object which wants to perform blocking operations at construction, by calling `bootstrap` in
/// the audio thread immediately after creation.
pub(crate) trait Bootstrap: Send + Sync {
    fn bootstrap(&self, _ctx: &syz::Context) -> Result<()> {
        Ok(())
    }
}
