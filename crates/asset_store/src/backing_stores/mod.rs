mod filesystem;

use std::io::Result as IoResult;

pub(crate) use filesystem::*;

type DirIter = Box<dyn Iterator<Item = IoResult<String>>>;

#[enum_dispatch::enum_dispatch(BackingStore)]
pub(crate) trait BackingStoreTrait {
    /// Read the "path" specified.
    fn open(&self, key: &str) -> IoResult<Box<dyn std::io::Read>>;

    /// Iterate over all keys in this store rooted at the specified subkey. For example "a/b" would yield 'a/b/c" but
    /// not "a/b" itself.
    fn iter_subdir(&self, prefix: &str) -> IoResult<DirIter>;

    /// Iterate over all keys in this store.
    fn iter_all(&self) -> IoResult<DirIter>;
}

#[enum_dispatch::enum_dispatch]
pub(crate) enum BackingStore {
    Filesystem(FilesystemStore),
}

impl BackingStore {
    pub(crate) fn new_filesystem(root: camino::Utf8PathBuf) -> IoResult<BackingStore> {
        Ok(BackingStore::Filesystem(FilesystemStore::new(root)?))
    }
}
