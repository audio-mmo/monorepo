use std::sync::Arc;

use anyhow::Result;
use synthizer as syz;

/// A trait providing I/O resources.
pub trait IoProvider: Send + Sync {
    fn decode_buffer(&self, key: &str) -> Result<Arc<syz::Buffer>>;
}
