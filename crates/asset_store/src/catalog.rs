/// A kind of asset.
///
/// Catalogs are subdirectories of an asset store.  They specify where assets live and what the behavior of the catalog should be.
pub trait Catalog: 'static + Send + Sync + std::cmp::Eq + std::cmp::Ord + std::hash::Hash {
    /// Get the subdirectory of this catalog.
    fn get_subdirectory(&self) -> &str;

    /// Return whether this catalog is localized.
    ///
    /// Localized catalogs are expected to be at `subdirectory/en-us/bla` where `bla` is the asset in the catalog.  This
    /// crate takes a localization and, for any localized catalog, will try to match that localization before falling
    /// back to the `en-us` version (this means that the `en-us` version must be present for the asset to count as
    /// existing).
    fn is_localized(&self) -> bool;
}
