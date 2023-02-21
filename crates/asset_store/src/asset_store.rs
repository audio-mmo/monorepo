use std::cell::RefCell;
use std::io::{Read, Result as IoResult};
use std::marker::PhantomData;

use camino::Utf8PathBuf;

use crate::backing_stores::*;
use crate::Catalog;

#[derive(Debug, thiserror::Error)]
pub enum AssetStoreOpenError {
    #[error("Io: {0}")]
    Io(#[from] std::io::Error),

    #[error("Unable to parse language tag: {0}")]
    LanguageTagParseError(#[from] language_tags::ParseError),

    #[error("Specified language tag is invalid: {0}")]
    LanguageTagValidationError(#[from] language_tags::ValidationError),
}

pub struct AssetStore<C: Catalog> {
    backing_store: BackingStore,
    _phantom: PhantomData<*const C>,
}

impl<C: Catalog> AssetStore<C> {
    pub fn open_fs(
        root: Utf8PathBuf,
        locale_tag: &str,
    ) -> Result<AssetStore<C>, AssetStoreOpenError> {
        let parsed = language_tags::LanguageTag::parse(locale_tag)?;
        parsed.validate()?;

        let backing_store = BackingStore::new_filesystem(root)?;
        Ok(AssetStore {
            backing_store,
            _phantom: PhantomData,
        })
    }

    /// This is the worst implementation of open possible: if the catalog is localized, then pretend we want en-us.
    pub fn open(&self, catalog: &C, key: &str) -> IoResult<Box<dyn Read>> {
        thread_local!(
            static WORKING_BUF: RefCell<Utf8PathBuf> = RefCell::new(Utf8PathBuf::new());
        );

        WORKING_BUF.with(|working_buf| {
            let mut working_buf = working_buf.borrow_mut();
            working_buf.clear();
            working_buf.push(catalog.get_subdirectory());
            if catalog.is_localized() {
                working_buf.push("en-us");
            }
            working_buf.push(key);
            self.backing_store.open(working_buf.as_str())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
    enum TestCatalog {
        Localized,
        Unlocalized,
    }

    impl Catalog for TestCatalog {
        fn get_subdirectory(&self) -> &str {
            match self {
                Self::Localized => "localized",
                Self::Unlocalized => "unlocalized",
            }
        }

        fn is_localized(&self) -> bool {
            matches!(self, Self::Localized)
        }
    }

    fn get_test_store() -> AssetStore<TestCatalog> {
        let fspath = format!("{}/test_assets/test_catalog", env!("CARGO_MANIFEST_DIR"));
        AssetStore::open_fs(fspath.into(), "en-us").expect("Could not open the asset store")
    }

    fn read_to_string(store: &AssetStore<TestCatalog>, catalog: &TestCatalog, key: &str) -> String {
        let mut opened = store.open(catalog, key).unwrap();
        let mut out = String::new();
        opened.read_to_string(&mut out).unwrap();
        out
    }

    #[test]
    fn test_basic_reading() {
        let store = get_test_store();
        assert_eq!(
            read_to_string(&store, &TestCatalog::Unlocalized, "unlocalized_data.txt"),
            "unlocalized"
        );
        assert_eq!(
            read_to_string(&store, &TestCatalog::Localized, "localized_data.txt"),
            "localized=en-us"
        );
    }
}
