use std::collections::BTreeMap;

use anyhow::Result;
use bytes::{BufMut, Bytes, BytesMut};
use itertools::Itertools;
use serde::{de::DeserializeOwned, Serialize};

use crate::{
    object_id::ObjectId,
    stores::{Meta, Store},
    version::Version,
};

/// Build patches for a store.
///
/// This type collects data from a store by gathering all entities for whom the version is greater than the input
/// version and for which a provided predicate returns true, serializing said entities to bytes.  It then provides ways
/// to get subsets of that data out as patches, which may then be applied to stores.
///
/// It is necessary to prepare multiple patches at once because patching is an expensive `O(n)` operation, and doing
/// them one by one is `O(P*N)` where `P` is players.
///
/// IMPORTANT: this only prepares patches for committed inserts, because it's intended to be called at the end of the
/// tick.
pub struct StorePatchBuilder {
    versions: BTreeMap<Version, Vec<PatchEntry>>,
}

#[derive(Clone)]
struct PatchEntry {
    object_id: ObjectId,
    version: Version,
    data: Bytes,
}

/// A patch, which may be applied to a given store.
pub struct StorePatch {
    entries: Vec<PatchEntry>,
}

impl StorePatchBuilder {
    /// Prepare to build patches for all data newer than the specified version and matching a predicate.
    ///
    /// If version isn't specified, prepare a patch for all versions.
    pub fn prepare<T: Serialize>(
        store: &Store<T, Version>,
        version: Option<Version>,
        mut predicate: impl FnMut(ObjectId) -> bool,
    ) -> StorePatchBuilder {
        // We build up one big buffer, then we split it at the end.  Otherwise the bytes crate is going to reallocate
        // over and over.
        let mut writer = BytesMut::new().writer();
        // Tuple is (version, id, start, end) where end is exclusive.
        let mut entries: Vec<(Version, ObjectId, usize, usize)> = vec![];

        for i in 0..store.index_len() {
            if let Meta::Alive(v) = store.meta[i] {
                let id = store.keys[i];
                if Some(v) > version && predicate(id) {
                    let start = writer.get_ref().len();
                    bincode::serialize_into(&mut writer, &store.values[i])
                        .expect("This is an infallible writer");
                    let end = writer.get_ref().len();
                    entries.push((v, id, start, end));
                }
            }
        }

        // Get the bytes:
        let buffer = writer.into_inner().freeze();
        // Sort our entries.
        entries.sort_unstable_by_key(|x| x.0);

        // Now we build up the BTreeMap.
        let versions = entries
            .into_iter()
            .group_by(|x| x.0)
            .into_iter()
            .map(|(v, group)| {
                let entries = group
                    .into_iter()
                    .map(|(_, object_id, start, end)| PatchEntry {
                        object_id,
                        version: v,
                        data: buffer.slice(start..end),
                    })
                    .collect();
                (v, entries)
            })
            .collect();

        StorePatchBuilder { versions }
    }

    /// Extract a patch of all objects whose version are newer than that specified and for which the predicate returns
    /// true.
    pub fn extract_patch(
        &self,
        version: Option<Version>,
        mut predicate: impl FnMut(ObjectId) -> bool,
    ) -> StorePatch {
        let min_ver = version.unwrap_or(Version::MIN);
        let mut entries = vec![];

        for (_, items) in self.versions.range(min_ver.increment()..=Version::MAX) {
            entries.extend(items.iter().filter(|x| predicate(x.object_id)).cloned());
        }

        // By sorting these, we ensure that they will be added to the resulting store in increasing order.  This
        // maximizes the efficiency, since stores are optimized for appending.
        entries.sort_unstable_by_key(|x| x.object_id);

        StorePatch { entries }
    }
}

impl StorePatch {
    /// Apply this patch to a store.
    ///
    /// Leaves the metadata at the greatest version used.  That is, if the patch is newer the version gets incremented.
    ///
    /// If the store isn't of the same type the patch was prepared for, then deserialization will likely fail.  If it
    /// doesn't, that's probably not what you want anyway.
    ///
    /// Also performs store maintenance under the assumption that we want to commit everything we just got.
    pub fn apply<T: DeserializeOwned>(&self, store: &mut Store<T, Version>) -> Result<()> {
        let original_meta = store.current_meta;

        for e in self.entries.iter() {
            let ver = store.meta_for_id(&e.object_id).cloned();
            if ver >= Some(e.version) {
                continue;
            }

            let blen = e.data.len();
            let deser = bincode::deserialize(&e.data[..blen])?;
            store.set_meta(e.version);
            store.insert(&e.object_id, deser);
        }

        // Now put the metadata back to what the user expects, if the store's current version was greater.
        if store.current_meta > original_meta {
            store.set_meta(original_meta);
        }

        // Commit and compact the store.
        store.maintenance();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store_to_vec(store: &Store<u64, Version>) -> Vec<(ObjectId, u64)> {
        store.iter().map(|x| (x.1, *x.2)).collect()
    }

    #[test]
    fn test_patch_empty() {
        let mut input_store: Store<u64, Version> = Store::new(Version::MIN);

        let o1 = ObjectId::new_testing(1);
        let o2 = ObjectId::new_testing(2);
        let o3 = ObjectId::new_testing(3);

        let mut version = Version::MIN;
        version = version.increment();

        input_store.set_meta(version);
        input_store.insert(&o1, 1);
        input_store.insert(&o2, 2);
        input_store.insert(&o3, 3);
        input_store.maintenance();

        let mut output_store: Store<u64, Version> = Store::new(version);
        let patch_builder = StorePatchBuilder::prepare(&input_store, None, |_| true);
        let patch = patch_builder.extract_patch(None, |_| true);
        patch.apply(&mut output_store).expect("Should apply");

        assert_eq!(store_to_vec(&output_store), vec![(o1, 1), (o2, 2), (o3, 3)]);
    }

    /// test extracting multiple patches for different versions from the same PatchBuilder.
    #[test]
    fn test_patch_multiversion() {
        let mut input_store: Store<u64, Version> = Store::new(Version::MIN);

        let o1 = ObjectId::new_testing(1);
        let o2 = ObjectId::new_testing(2);
        let o3 = ObjectId::new_testing(3);

        // Don't use MIN, MIN is too easy.
        let mut version = Version::MIN;
        version = version.increment();
        input_store.set_meta(version);
        input_store.insert(&o1, 1);
        version = version.increment();
        input_store.set_meta(version);
        input_store.insert(&o2, 2);
        version = version.increment();
        input_store.set_meta(version);
        input_store.insert(&o3, 3);
        input_store.maintenance();

        let patch_builder = StorePatchBuilder::prepare(&input_store, None, |_| true);

        {
            let mut output_store: Store<u64, Version> = Store::new(Version::MIN);
            let patch = patch_builder.extract_patch(Some(Version::MIN), |_| true);
            patch.apply(&mut output_store).expect("Should apply");

            assert_eq!(store_to_vec(&output_store), vec![(o1, 1), (o2, 2), (o3, 3)]);
        }

        {
            let mut output_store: Store<u64, Version> = Store::new(Version::MIN);
            let patch = patch_builder.extract_patch(Some(Version::MIN.increment()), |_| true);
            patch.apply(&mut output_store).expect("Should apply");

            assert_eq!(store_to_vec(&output_store), vec![(o2, 2), (o3, 3)]);
        }

        {
            let mut output_store: Store<u64, Version> = Store::new(Version::MIN);
            let patch =
                patch_builder.extract_patch(Some(Version::MIN.increment().increment()), |_| true);
            patch.apply(&mut output_store).expect("Should apply");

            assert_eq!(store_to_vec(&output_store), vec![(o3, 3)]);
        }

        {
            let mut output_store: Store<u64, Version> = Store::new(Version::MIN);
            let patch = patch_builder.extract_patch(Some(Version::MIN), |_| true);
            patch.apply(&mut output_store).expect("Should apply");

            assert_eq!(store_to_vec(&output_store), vec![(o1, 1), (o2, 2), (o3, 3)]);
        }

        {
            let mut output_store: Store<u64, Version> = Store::new(Version::MIN);
            let patch = patch_builder.extract_patch(
                Some(Version::MIN.increment().increment().increment()),
                |_| true,
            );
            patch.apply(&mut output_store).expect("Should apply");

            assert_eq!(store_to_vec(&output_store), vec![]);
        }
    }

    /// test filtering out in the builder's prepare method.
    #[test]
    fn test_filtering_in_builder() {
        let mut input_store: Store<u64, Version> = Store::new(Version::MIN);

        let o1 = ObjectId::new_testing(1);
        let o2 = ObjectId::new_testing(2);
        let o3 = ObjectId::new_testing(3);

        // Don't use MIN, MIN is too easy.
        let version = Version::MIN.increment();

        input_store.set_meta(version);
        input_store.insert(&o1, 1);
        input_store.insert(&o2, 2);
        input_store.insert(&o3, 3);
        input_store.maintenance();

        let pb = StorePatchBuilder::prepare(&input_store, None, |o| o == o2 || o == o3);
        let patch = pb.extract_patch(None, |_| true);
        let mut output_store: Store<u64, Version> = Store::new(version);
        patch.apply(&mut output_store).expect("Should apply");
        assert_eq!(store_to_vec(&output_store), vec![(o2, 2), (o3, 3)]);
    }

    /// test filtering out in extract.
    #[test]
    fn test_filtering_in_extraction() {
        let mut input_store: Store<u64, Version> = Store::new(Version::MIN);

        let o1 = ObjectId::new_testing(1);
        let o2 = ObjectId::new_testing(2);
        let o3 = ObjectId::new_testing(3);

        // Don't use MIN, MIN is too easy.
        let version = Version::MIN.increment();

        input_store.set_meta(version);
        input_store.insert(&o1, 1);
        input_store.insert(&o2, 2);
        input_store.insert(&o3, 3);
        input_store.maintenance();

        let pb = StorePatchBuilder::prepare(&input_store, None, |_| true);
        let patch = pb.extract_patch(None, |o| o == o2 || o == o3);
        let mut output_store: Store<u64, Version> = Store::new(version);
        patch.apply(&mut output_store).expect("Should apply");
        assert_eq!(store_to_vec(&output_store), vec![(o2, 2), (o3, 3)]);
    }

    /// Test that we don't override newer values in the destination.
    #[test]
    fn test_no_override_newer() {
        let mut input_store: Store<u64, Version> = Store::new(Version::MIN);

        let o1 = ObjectId::new_testing(1);
        let o2 = ObjectId::new_testing(2);
        let o3 = ObjectId::new_testing(3);

        // Don't use MIN, MIN is too easy.
        let version = Version::MIN.increment();

        input_store.set_meta(version);
        input_store.insert(&o1, 1);
        input_store.insert(&o2, 2);
        input_store.insert(&o3, 3);
        input_store.maintenance();

        let pb = StorePatchBuilder::prepare(&input_store, None, |_| true);
        let patch = pb.extract_patch(None, |_| true);
        let mut output_store: Store<u64, Version> = Store::new(version);
        output_store.set_meta(version.increment_multi(5));
        output_store.insert(&o1, 5);
        patch.apply(&mut output_store).expect("Should apply");
        assert_eq!(store_to_vec(&output_store), vec![(o1, 5), (o2, 2), (o3, 3)]);
    }

    /// Test that we only extract the subset we request when preparing to build patches (e.g. version in prepare).
    #[test]
    fn test_version_in_prepare() {
        let mut input_store: Store<u64, Version> = Store::new(Version::MIN);

        let o1 = ObjectId::new_testing(1);
        let o2 = ObjectId::new_testing(2);
        let o3 = ObjectId::new_testing(3);

        // Don't use MIN, MIN is too easy.
        let mut version = Version::MIN.increment();

        input_store.set_meta(version);
        input_store.insert(&o1, 1);
        version = version.increment();
        input_store.set_meta(version);
        input_store.insert(&o2, 2);
        version = version.increment();
        input_store.set_meta(version);
        input_store.insert(&o3, 3);
        input_store.maintenance();

        let pb =
            StorePatchBuilder::prepare(&input_store, Some(Version::MIN.increment_multi(2)), |_| {
                true
            });
        let patch = pb.extract_patch(None, |_| true);
        let mut output_store: Store<u64, Version> = Store::new(version);
        patch.apply(&mut output_store).expect("Should apply");
        assert_eq!(store_to_vec(&output_store), vec![(o3, 3)]);
    }
}
