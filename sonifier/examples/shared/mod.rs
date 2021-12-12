use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use asset_lru::{AssetCache, FilesystemVfs};
use synthizer as syz;

pub struct IoProviderImpl {
    cache: AssetCache<FilesystemVfs, syz::BufferAssetLruDecoder>,
}

impl IoProviderImpl {
    pub fn new(root_path: &Path) -> Result<IoProviderImpl> {
        Ok(IoProviderImpl {
            cache: AssetCache::new(
                FilesystemVfs::new(root_path)?,
                syz::BufferAssetLruDecoder::new(),
                asset_lru::AssetCacheConfig {
                    max_bytes_cost: 10000000,
                    max_decoded_cost: 100000000,
                    max_single_object_bytes_cost: 100000000,
                    max_single_object_decoded_cost: 100000000,
                },
            ),
        })
    }
}

impl ammo_sonifier::IoProvider for IoProviderImpl {
    fn decode_buffer(&self, key: &str) -> Result<Arc<syz::Buffer>> {
        Ok(self.cache.get(key)?)
    }
}
