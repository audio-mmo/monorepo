use std::{collections::BTreeMap, marker::PhantomData};

use crate::object_id::ObjectId;

pub struct Store<T, M> {
    keys: Vec<ObjectId>,
    values: Vec<T>,
    meta: Vec<Meta<M>>,
    pending_inserts: BTreeMap<ObjectId, PendingInsertRecord<T, M>>,
    current_meta: M,
}

#[derive(Debug, Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Hash)]
enum Meta<M> {
    Alive(M),
    Dead,
}

struct PendingInsertRecord<T, M> {
    value: T,
    meta: Meta<M>,
}

pub struct StoreRef<'a, T, M> {
    value: &'a T,
    meta: PhantomData<&'a M>,
}

pub struct StoreRefMut<'a, T, M: Clone> {
    current_meta: &'a M,
    value: &'a mut T,
    meta: &'a mut Meta<M>,
    /// Whether this mutable reference was actually mutated, or just constructed.
    ///
    /// Set to true in DerefMut.
    mutated: bool,
}

impl<'a, T, M> std::ops::Deref for StoreRef<'a, T, M> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'a, T, M: Clone> std::ops::Deref for StoreRefMut<'a, T, M> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &*self.value
    }
}

impl<'a, T, M: Clone> std::ops::DerefMut for StoreRefMut<'a, T, M> {
    fn deref_mut(&mut self) -> &mut <Self as std::ops::Deref>::Target {
        self.mutated = true;
        self.value
    }
}

impl<'a, T, M: Clone> Drop for StoreRefMut<'a, T, M> {
    fn drop(&mut self) {
        if self.mutated {
            // Be careful: there are ways to get borrows of dead elements when getting by raw index or deleting through
            // a StoreRefMut.  Don't resurrect these.
            if self.meta.is_alive() {
                *self.meta = Meta::Alive((*self.current_meta).clone());
            }
        }
    }
}

impl<'a, T, M: Clone> StoreRefMut<'a, T, M> {
    /// Delete this object from the store.
    pub fn delete(self) {
        *self.meta = Meta::Dead;
    }
}
impl<M> Meta<M> {
    fn is_alive(&self) -> bool {
        matches!(self, Meta::Alive(_))
    }

    fn is_dead(&self) -> bool {
        !self.is_alive()
    }

    fn get_alive_meta(&self) -> Option<&M> {
        match self {
            Meta::Alive(m) => Some(m),
            _ => None,
        }
    }
}

impl<T, M: Default> Default for Store<T, M> {
    fn default() -> Self {
        Self {
            keys: vec![],
            values: vec![],
            meta: vec![],
            current_meta: Default::default(),
            pending_inserts: Default::default(),
        }
    }
}

/// Internal enum for the possible results from doing a search for an index given an id: we found it, we need to insert
/// before the specified index, or we found a tombstone.
enum SearchResult {
    Found(usize),
    InsertBefore(usize),
    TombstoneAvailable(usize),
    Pending,
}

impl<T, M: Clone> Store<T, M> {
    pub fn new(initial_meta: M) -> Self {
        Store {
            keys: vec![],
            values: vec![],
            meta: vec![],
            current_meta: initial_meta,
            pending_inserts: Default::default(),
        }
    }

    /// Update the metadata which will be applied to any object which changes after this call.
    pub fn set_meta(&mut self, meta: M) {
        self.current_meta = meta;
    }

    fn compact(&mut self) {
        let mut key_ind = 0;
        self.keys.retain(|_| {
            let ret = self.meta[key_ind].is_alive();
            key_ind += 1;
            ret
        });

        let mut val_ind = 0;
        self.values.retain(|_| {
            let ret = self.meta[val_ind].is_alive();
            val_ind += 1;
            ret
        });

        self.meta
            .resize_with(self.keys.len(), || panic!("Shrinking"));
    }

    pub fn maintenance(&mut self) {
        // Compact after handling pending inserts, since this will allow reuse of tombstones.
        self.commit_pending_inserts();
        self.compact();
        self.keys.shrink_to_fit();
        self.values.shrink_to_fit();
        self.meta.shrink_to_fit();
    }

    fn search_index_from_id(&self, id: &ObjectId) -> SearchResult {
        let ind = self.keys.binary_search(id);
        match ind {
            Ok(i) | Err(i) if i < self.meta.len() && self.meta[i].is_dead() => {
                SearchResult::TombstoneAvailable(i)
            }
            // be careful to check for liveness here; StoreRefMut doesn't remove from pending inserts.
            Err(_)
                if self
                    .pending_inserts
                    .get(id)
                    .map(|x| x.meta.is_alive())
                    .unwrap_or(false) =>
            {
                SearchResult::Pending
            }
            Ok(i) => SearchResult::Found(i),
            Err(i) => {
                // Special case: if the end of the vector is a tombstone and we would insert at the end, use that.
                if let Some(t) = self.meta.last() {
                    if t.is_dead() && i == self.meta.len() {
                        return SearchResult::TombstoneAvailable(self.meta.len() - 1);
                    }
                }
                SearchResult::InsertBefore(i)
            }
        }
    }

    pub fn insert(&mut self, id: &ObjectId, val: T) -> Option<T> {
        match self.search_index_from_id(id) {
            SearchResult::Found(i) => {
                let mut old = val;
                std::mem::swap(&mut self.values[i], &mut old);
                self.meta[i] = Meta::Alive(self.current_meta.clone());
                Some(old)
            }
            SearchResult::TombstoneAvailable(i) => {
                self.meta[i] = Meta::Alive(self.current_meta.clone());
                self.keys[i] = *id;
                self.values[i] = val;
                None
            }
            SearchResult::InsertBefore(i) => {
                // If i is at the end of the vector, fast case it.
                if i == self.keys.len() {
                    self.keys.push(*id);
                    self.values.push(val);
                    self.meta.push(Meta::Alive(self.current_meta.clone()));
                    return None;
                }
                self.pending_inserts
                    .insert(
                        *id,
                        PendingInsertRecord {
                            meta: Meta::Alive(self.current_meta.clone()),
                            value: val,
                        },
                    )
                    .map(|x| x.value)
            }
            SearchResult::Pending => self
                .pending_inserts
                .insert(
                    *id,
                    PendingInsertRecord {
                        meta: Meta::Alive(self.current_meta.clone()),
                        value: val,
                    },
                )
                .map(|x| x.value),
        }
    }

    pub fn commit_pending_inserts(&mut self) {
        // Now that we've reused what tombstones we can, get rid of the rest.
        self.compact();

        // It is possible to get dead elements if the user deletes via a StoreRefMut.
        let mut iterator = std::mem::take(&mut self.pending_inserts)
            .into_iter()
            .filter(|x| x.1.meta.is_alive());

        // While we're not just pushing to the end, do some looping.
        for (k, v) in &mut iterator {
            // The fast and common case is that we're just pushing to the end. Break out once we detect this.
            if let Some(l) = self.keys.last().cloned() {
                if k > l {
                    // k is consumed, we must deal with it here because we can't put it back on the iterator.
                    self.keys.push(k);
                    self.values.push(v.value);
                    self.meta.push(v.meta);
                    break;
                }
            }

            match self.keys.binary_search(&k) {
                Ok(i) => {
                    self.values[i] = v.value;
                    self.meta[i] = Meta::Alive(self.current_meta.clone());
                }
                Err(ind) if ind < self.meta.len() && self.meta[ind].is_dead() => {
                    self.keys[ind] = k;
                    self.values[ind] = v.value;
                    self.meta[ind] = Meta::Alive(self.current_meta.clone());
                }
                Err(ind) => {
                    self.keys.insert(ind, k);
                    self.values.insert(ind, v.value);
                    self.meta
                        .insert(ind, Meta::Alive(self.current_meta.clone()));
                }
            }
        }

        // Most of the work actually happens here: the special cases above were just getting things out of the way.
        // Object id generationguarantees ordering per run, so the only times we really see the above cases are at
        // startup if the clock went backward or when loading from saved data.
        for (k, v) in iterator {
            self.keys.push(k);
            self.values.push(v.value);
            self.meta.push(v.meta);
        }

        assert_eq!(self.keys.len(), self.values.len());
        assert_eq!(self.values.len(), self.meta.len());
    }

    pub fn get_by_id(&self, id: &ObjectId) -> Option<StoreRef<T, M>> {
        match self.search_index_from_id(id) {
            SearchResult::Found(i) => self.values.get(i).map(|value| StoreRef {
                value,
                meta: PhantomData,
            }),
            SearchResult::TombstoneAvailable(_) | SearchResult::InsertBefore(_) => None,
            SearchResult::Pending => self.pending_inserts.get(id).map(|x| StoreRef {
                value: &x.value,
                meta: PhantomData,
            }),
        }
    }

    pub fn get_by_id_mut(&mut self, id: &ObjectId) -> Option<StoreRefMut<T, M>> {
        match self.search_index_from_id(id) {
            SearchResult::Found(i) => Some(StoreRefMut {
                current_meta: &self.current_meta,
                value: &mut self.values[i],
                meta: &mut self.meta[i],
                mutated: false,
            }),
            SearchResult::TombstoneAvailable(_) | SearchResult::InsertBefore(_) => None,
            SearchResult::Pending => self.pending_inserts.get_mut(id).map(|x| StoreRefMut {
                current_meta: &self.current_meta,
                value: &mut x.value,
                meta: &mut x.meta,
                mutated: false,
            }),
        }
    }

    pub fn get_by_index(&self, index: usize) -> Option<StoreRef<T, M>> {
        self.values.get(index).map(|value| StoreRef {
            value,
            meta: PhantomData,
        })
    }

    pub fn get_by_index_mut(&mut self, index: usize) -> Option<StoreRefMut<T, M>> {
        self.values.get_mut(index).map(|value| StoreRefMut {
            current_meta: &self.current_meta,
            value,
            meta: &mut self.meta[index],
            mutated: false,
        })
    }

    pub fn index_for_id(&self, id: &ObjectId) -> Option<usize> {
        match self.search_index_from_id(id) {
            SearchResult::Found(i) => Some(i),
            _ => None,
        }
    }

    pub fn delete_id(&mut self, id: &ObjectId) -> bool {
        match self.search_index_from_id(id) {
            SearchResult::Found(i) => {
                self.meta[i] = Meta::Dead;
                true
            }
            SearchResult::Pending => {
                self.pending_inserts
                    .remove(id)
                    .expect("Should havve been in pending inserts");
                true
            }
            SearchResult::InsertBefore(_) | SearchResult::TombstoneAvailable(_) => false,
        }
    }

    pub fn delete_index(&mut self, index: usize) -> bool {
        let ret = self.meta[index].is_alive();
        self.meta[index] = Meta::Dead;
        ret
    }

    pub fn is_index_alive(&self, index: usize) -> bool {
        self.meta[index].is_alive()
    }

    pub fn index_len(&self) -> usize {
        self.keys.len()
    }

    pub fn id_at_index(&self, index: usize) -> ObjectId {
        self.keys[index]
    }

    /// Returns a `(index, ObjectId, value)` iterator.
    pub fn iter(&self) -> impl Iterator<Item = (usize, ObjectId, StoreRef<T, M>)> {
        self.keys
            .iter()
            .enumerate()
            .zip(self.values.iter())
            .filter_map(move |((i, k), v)| {
                if self.meta[i].is_dead() {
                    return None;
                }
                Some((
                    i,
                    *k,
                    StoreRef {
                        value: v,
                        meta: PhantomData,
                    },
                ))
            })
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (usize, ObjectId, StoreRefMut<T, M>)> {
        let ks = &self.keys;
        let vs = &mut self.values;
        let ms = &mut self.meta;
        let current_meta = &self.current_meta;
        ks.iter()
            .enumerate()
            .zip(vs.iter_mut())
            .zip(ms.iter_mut())
            .filter_map(move |(((i, k), v), m)| {
                if m.is_dead() {
                    return None;
                }
                Some((
                    i,
                    *k,
                    StoreRefMut {
                        value: v,
                        current_meta,
                        meta: m,
                        mutated: false,
                    },
                ))
            })
    }

    /// Get an id's metadata, if that id is alive.
    pub fn meta_for_id(&self, id: &ObjectId) -> Option<&M> {
        self.meta[self.index_for_id(id)?].get_alive_meta()
    }

    /// Get metadata for a given index.
    ///
    /// Unlike get_by_index, this can't return metadata for dead indices.
    pub fn meta_for_index(&self, index: usize) -> Option<&M> {
        self.meta.get(index)?.get_alive_meta()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_ordered_inserting() {
        let mut store: Store<u64, ()> = Store::new(());
        for i in 1..=5 {
            store.insert(&ObjectId::new_testing(i), i);
        }

        // The index portion should contain everything.
        assert_eq!(store.index_len(), 5);
        assert_eq!(store.pending_inserts.len(), 0);

        assert_eq!(
            &store.keys,
            &vec![
                ObjectId::new_testing(1),
                ObjectId::new_testing(2),
                ObjectId::new_testing(3),
                ObjectId::new_testing(4),
                ObjectId::new_testing(5)
            ]
        );

        for i in 0..5 {
            assert_eq!(*store.get_by_index(i).unwrap(), ((i + 1) as u64));
        }
    }

    #[test]
    fn insert_reusing_tombstones() {
        let mut store: Store<u64, ()> = Store::new(());
        for i in 1..=5 {
            // Leave some holes.
            store.insert(&ObjectId::new_testing(i * 2), i * 2);
        }
        store.commit_pending_inserts();

        // Do some deletes and replaces.
        store.delete_index(2);
        assert!(!store.is_index_alive(2));
        store.insert(&ObjectId::new_testing(5), 5);
        assert_eq!(store.id_at_index(2), ObjectId::new_testing(5));
        assert_eq!(*store.get_by_index(2).unwrap(), 5);

        store.delete_index(1);
        store.delete_index(3);
        assert!(!store.is_index_alive(1));
        assert!(!store.is_index_alive(3));

        store.insert(&ObjectId::new_testing(3), 3);
        store.insert(&ObjectId::new_testing(7), 7);
        assert_eq!(store.id_at_index(1), ObjectId::new_testing(3));
        assert_eq!(*store.get_by_index(1).unwrap(), 3);
        assert!(store.is_index_alive(1));
        assert_eq!(store.id_at_index(3), ObjectId::new_testing(7));
        assert_eq!(*store.get_by_index(3).unwrap(), 7);
        assert!(store.is_index_alive(3));

        // Reuse of the first and last elements are an interesting case worth doing.
        store.delete_index(0);
        assert!(!store.is_index_alive(0));
        store.insert(&ObjectId::new_testing(1), 1);
        assert!(store.is_index_alive(0));
        assert_eq!(*store.get_by_index(0).unwrap(), 1);
        assert_eq!(store.id_at_index(0), ObjectId::new_testing(1));

        store.delete_index(4);
        assert!(!store.is_index_alive(4));
        // We can use any object id here.
        store.insert(&ObjectId::new_testing(100), 100);
        assert!(store.is_index_alive(4));
        assert_eq!(store.id_at_index(4), ObjectId::new_testing(100));
        assert_eq!(*store.get_by_index(4).unwrap(), 100);
    }

    #[test]
    fn test_pending_inserts() {
        let mut store: Store<u64, ()> = Store::new(());

        store.insert(&ObjectId::new_testing(1), 1);
        store.insert(&ObjectId::new_testing(5), 5);
        store.insert(&ObjectId::new_testing(2), 2);
        store.insert(&ObjectId::new_testing(3), 3);
        store.insert(&ObjectId::new_testing(4), 4);

        assert_eq!(store.index_len(), 2);
        assert_eq!(store.pending_inserts.len(), 3);
        store.commit_pending_inserts();

        assert_eq!(
            &store.keys,
            &vec![
                ObjectId::new_testing(1),
                ObjectId::new_testing(2),
                ObjectId::new_testing(3),
                ObjectId::new_testing(4),
                ObjectId::new_testing(5)
            ]
        );
        assert_eq!(&store.values, &vec![1, 2, 3, 4, 5]);
        assert_eq!(&store.meta, &vec![Meta::Alive(()); 5]);
    }

    #[test]
    fn test_mutation() {
        let mut store: Store<u64, ()> = Store::new(());

        store.insert(&ObjectId::new_testing(1), 1);
        store.insert(&ObjectId::new_testing(5), 5);
        store.insert(&ObjectId::new_testing(2), 2);
        store.insert(&ObjectId::new_testing(3), 3);
        store.insert(&ObjectId::new_testing(4), 4);

        store.insert(&ObjectId::new_testing(1), 11);
        store.insert(&ObjectId::new_testing(5), 15);
        store.insert(&ObjectId::new_testing(2), 12);
        store.insert(&ObjectId::new_testing(3), 13);
        store.insert(&ObjectId::new_testing(4), 14);

        assert_eq!(
            &store.keys,
            &vec![ObjectId::new_testing(1), ObjectId::new_testing(5)]
        );
        assert_eq!(&store.values, &vec![11, 15]);
        assert_eq!(
            store
                .pending_inserts
                .iter()
                .map(|x| (*x.0, x.1.value))
                .collect::<Vec<_>>(),
            vec![
                (ObjectId::new_testing(2), 12),
                (ObjectId::new_testing(3), 13),
                (ObjectId::new_testing(4), 14)
            ]
        );

        store.commit_pending_inserts();

        assert_eq!(
            &store.keys,
            &vec![
                ObjectId::new_testing(1),
                ObjectId::new_testing(2),
                ObjectId::new_testing(3),
                ObjectId::new_testing(4),
                ObjectId::new_testing(5)
            ]
        );
        assert_eq!(&store.values, &vec![11, 12, 13, 14, 15]);
        assert_eq!(&store.meta, &vec![Meta::Alive(()); 5],);
    }

    #[test]
    fn test_get_by_id() {
        let mut store: Store<u64, ()> = Store::new(());
        // Leave some holes.
        for i in 1..=10 {
            store.insert(&ObjectId::new_testing(i * 2), i * 2);
        }

        for i in 1..=10 {
            assert_eq!(
                *store.get_by_id(&ObjectId::new_testing(i * 2)).unwrap(),
                i * 2,
            );
            assert_eq!(
                *store.get_by_id_mut(&ObjectId::new_testing(i * 2)).unwrap(),
                i * 2
            );

            // Check that any index we shouldn't have isn't present.
            assert!(store.get_by_id(&ObjectId::new_testing(i * 2 + 1)).is_none());
            assert!(store
                .get_by_id_mut(&ObjectId::new_testing(i * 2 + 1))
                .is_none());
        }

        // Putting something in the pending inserts should work.
        store.insert(&ObjectId::new_testing(1), 1);
        assert_eq!(*store.get_by_id(&ObjectId::new_testing(1)).unwrap(), 1);
        assert_eq!(*store.get_by_id_mut(&ObjectId::new_testing(1)).unwrap(), 1);
    }

    #[test]
    fn test_compaction() {
        let mut store: Store<u64, ()> = Store::new(());
        for i in 1..=5 {
            store.insert(&ObjectId::new_testing(i), i);
        }

        store.commit_pending_inserts();
        assert!(store.delete_id(&ObjectId::new_testing(2)));
        assert!(store.delete_id(&ObjectId::new_testing(4)));
        assert!(store.delete_id(&ObjectId::new_testing(5)));

        assert_eq!(
            &store.keys,
            &vec![
                ObjectId::new_testing(1),
                ObjectId::new_testing(2),
                ObjectId::new_testing(3),
                ObjectId::new_testing(4),
                ObjectId::new_testing(5)
            ]
        );
        assert_eq!(&store.values, &vec![1, 2, 3, 4, 5]);
        assert_eq!(
            &store.meta,
            &vec![
                Meta::Alive(()),
                Meta::Dead,
                Meta::Alive(()),
                Meta::Dead,
                Meta::Dead,
            ]
        );
    }

    #[test]
    fn test_iter() {
        let mut store: Store<u64, ()> = Store::new(());

        for i in 1..=5 {
            store.insert(&ObjectId::new_testing(i), i);
        }

        store.maintenance();
        store.delete_index(2);

        let res = store
            .iter()
            .map(|i| (i.0, i.1.get_counter(), *i.2))
            .collect::<Vec<_>>();
        assert_eq!(res, vec![(0, 1, 1), (1, 2, 2), (3, 4, 4), (4, 5, 5)]);
    }

    /// We need to also test `iter_mut` because the implementation of it is entirely different from `iter`.
    #[test]
    fn test_iter_mut() {
        let mut store: Store<u64, ()> = Store::new(());

        for i in 1..=5 {
            store.insert(&ObjectId::new_testing(i), i);
        }

        store.maintenance();
        store.delete_index(2);

        let res = store
            .iter_mut()
            .map(|i| (i.0, i.1.get_counter(), *i.2))
            .collect::<Vec<_>>();
        assert_eq!(res, vec![(0, 1, 1), (1, 2, 2), (3, 4, 4), (4, 5, 5)]);
    }

    #[test]
    fn test_basic_meta() {
        let mut store: Store<u64, u64> = Store::new(0);
        store.insert(&ObjectId::new_testing(1), 0);
        assert_eq!(*store.meta_for_index(0).unwrap(), 0);
        store.set_meta(1);
        *store.get_by_id_mut(&ObjectId::new_testing(1)).unwrap() = 1;
        assert_eq!(*store.meta_for_index(0).unwrap(), 1);
        assert_eq!(*store.get_by_id(&ObjectId::new_testing(1)).unwrap(), 1);
    }

    #[test]
    fn test_meta_appending() {
        let mut store: Store<u64, u64> = Store::new(0);
        for i in 1..=5 {
            store.set_meta(i);
            store.insert(&ObjectId::new_testing(i), i);
        }

        assert_eq!(
            &store.keys,
            &vec![
                ObjectId::new_testing(1),
                ObjectId::new_testing(2),
                ObjectId::new_testing(3),
                ObjectId::new_testing(4),
                ObjectId::new_testing(5)
            ]
        );
        assert_eq!(&store.values, &vec![1, 2, 3, 4, 5]);
        assert_eq!(
            &store.meta,
            &vec![
                Meta::Alive(1),
                Meta::Alive(2),
                Meta::Alive(3),
                Meta::Alive(4),
                Meta::Alive(5),
            ]
        );
    }

    #[test]
    fn test_deletion_through_ref() {
        let mut store: Store<u64, u64> = Store::new(0);

        // We need to insert backward, otherwise we don't have anything in the pending inserts to test with.
        store.insert(&ObjectId::new_testing(2), 2);
        store.insert(&ObjectId::new_testing(1), 1);

        assert!(!store.pending_inserts.is_empty());

        // Now, delete both by grabbing their refs.
        store
            .get_by_id_mut(&ObjectId::new_testing(1))
            .unwrap()
            .delete();
        store
            .get_by_id_mut(&ObjectId::new_testing(2))
            .unwrap()
            .delete();

        assert!(store.get_by_id(&ObjectId::new_testing(1)).is_none());
        assert!(store.get_by_id(&ObjectId::new_testing(2)).is_none());

        assert!(store.get_by_id_mut(&ObjectId::new_testing(1)).is_none());
        assert!(store.get_by_id_mut(&ObjectId::new_testing(2)).is_none());

        assert!(store
            .pending_inserts
            .get(&ObjectId::new_testing(1))
            .unwrap()
            .meta
            .is_dead());
    }

    #[test]
    fn mutating_changes_meta_on_drop() {
        let mut store: Store<u64, u64> = Store::new(0);
        store.insert(&ObjectId::new_testing(1), 1);
        assert_eq!(*store.meta_for_index(0).unwrap(), 0);

        store.set_meta(1);
        // Just grabbing it and immediately dropping it does nothing.
        assert!(store.get_by_id_mut(&ObjectId::new_testing(1)).is_some());
        assert_eq!(*store.meta_for_id(&ObjectId::new_testing(1)).unwrap(), 0);

        // But actually mutating does.
        *store.get_by_id_mut(&ObjectId::new_testing(1)).unwrap() = 5;
        assert_eq!(*store.get_by_id(&ObjectId::new_testing(1)).unwrap(), 5);
        assert_eq!(*store.meta_for_id(&ObjectId::new_testing(1)).unwrap(), 1);
    }
}
