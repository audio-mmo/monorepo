use std::io::{Read, Result as IoResult};

use camino::Utf8PathBuf;

use crate::backing_stores::*;

pub struct AssetStore {
    backing_store: BackingStore,
}

impl AssetStore {
    pub fn open_fs(root: Utf8PathBuf) -> std::io::Result<AssetStore> {
        let backing_store = BackingStore::new_filesystem(root)?;
        Ok(AssetStore { backing_store })
    }

    pub fn open(&self, key: &str) -> IoResult<Box<dyn Read>> {
        self.backing_store.open(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_store() -> AssetStore {
        let fspath = format!("{}/test_assets/test_catalog", env!("CARGO_MANIFEST_DIR"));
        AssetStore::open_fs(fspath.into()).expect("Could not open the asset store")
    }

    fn read_to_string(store: &AssetStore, key: &str) -> String {
        let mut opened = store.open(key).unwrap();
        let mut out = String::new();
        opened.read_to_string(&mut out).unwrap();
        out
    }

    #[test]
    fn test_basic_reading() {
        let store = get_test_store();
        assert_eq!(read_to_string(&store, "folder/data.txt"), "hello");
    }
}
