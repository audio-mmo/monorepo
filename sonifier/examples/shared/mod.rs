use std::io::{Read, Seek};
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use asset_lru::{AssetCache, FilesystemVfs};
use synthizer as syz;

pub struct IoProviderImpl {
    cache: AssetCache<Arc<FilesystemVfs>, syz::BufferAssetLruDecoder>,
    vfs: Arc<FilesystemVfs>,
}

impl IoProviderImpl {
    pub fn new(root_path: &Path) -> Result<IoProviderImpl> {
        let vfs = Arc::new(FilesystemVfs::new(root_path)?);
        Ok(IoProviderImpl {
            cache: AssetCache::new(
                vfs.clone(),
                syz::BufferAssetLruDecoder::new(),
                asset_lru::AssetCacheConfig {
                    max_bytes_cost: 10000000,
                    max_decoded_cost: 100000000,
                    max_single_object_bytes_cost: 100000000,
                    max_single_object_decoded_cost: 100000000,
                },
            ),
            vfs,
        })
    }
}

struct Stream(std::fs::File);

impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf)
    }
}

impl Seek for Stream {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.0.seek(pos)
    }
}

impl syz::CloseStream for Stream {
    fn close(&mut self) -> std::result::Result<(), Box<dyn std::fmt::Display>> {
        // Files just close when dropped.
        Ok(())
    }
}

impl ammo_sonifier::IoProvider for IoProviderImpl {
    fn decode_buffer(&self, key: &str) -> Result<Arc<syz::Buffer>> {
        Ok(self.cache.get(key)?)
    }

    fn get_stream_handle(&self, key: &str) -> Result<syz::StreamHandle> {
        let f = self.vfs.open_file(Path::new(key))?;
        let def = syz::CustomStreamDef::from_seekable(Stream(f))?;
        Ok(syz::StreamHandle::from_stream_def(def)?)
    }
}
