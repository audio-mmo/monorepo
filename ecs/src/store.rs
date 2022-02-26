//! The [Store] is a store optimized for in-order object ids, using 3 vecs and a BTreeMap.  See the comments on the
//! [Store] type for details.
use std::collections::BTreeMap;

use crate::object_id::ObjectId;

/// A store for component data consisting of some vecs.
///
/// This implementation uses:
///
/// - A vec of keys, which must be [ObjectId]s.
/// - A vec of values, which can be anything.
/// - A vec of metadata, which records whether or not things are alive.
///
/// This container may be accessed by index (for iterating) or by object id (for map-like access).  When objects are
/// inserted, they go either to an unused slot in the vecs or to a queue of pending inserts.  Like with stdlib maps,
/// inserting twice is how one can override the key, but a variety of `get` and `get_mut` methods are available.
///
/// Observe the following rules about visibility:
///
/// - Indices are stable until inserts are committed, but the vec may grow and/or reuse tombstone-occupied cells.
/// - All deletes are visible if going through the `by_id` interfaces.
/// - All inserts are visible if going through the `by_id` interfaces.
/// - Objects may not be assigned an index until after `commit_pending_inserts` is called.  They will usually be if
///   objects are inserted in order of increasing id.
/// - The `by_index` API doesn't check tombstones for you.  Use `is_index_alive` for that.
/// - Data isn't dropped until inserts are committed and/or maintenance is called.
///
/// So the usage pattern is: when iterating if you're not inserting do nothing, otherwise you might or might not see the
/// object.  In common usage you probably will but this isn't guaranteed.  If you want to see the object commit all the
/// inserts.
///
/// A method `maintenance` should periodically be called: this gets rid of tombstones, compacts the vectors, and commits
/// pending inserts.  Failure to call this method periodically will slowly grow the vecs to the largest size the
/// container has evern been and keep them there, and also greatly slow iteration which must skip tombstones.
/// The design here is optimized for the case in which we want to join multiple stores to perform queries in `O(1)`
/// additional memory and `O(n)` time.  Basically, when deleting/inserting in higher level components, insert/deletions
/// may not be visible until the next tick, but modifications are visible immediately.  
///
/// Methods which don't return `Option` use normal `[]` indexing under the hood and generally panic on invariant
/// failures.
pub struct Store<T> {
    keys: Vec<ObjectId>,
    values: Vec<T>,
    meta: Vec<Meta>,
    pending_inserts: BTreeMap<ObjectId, T>,
}

#[derive(Debug, Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Hash)]
enum Meta {
    Alive,
    Dead,
}

impl Meta {
    fn is_alive(&self) -> bool {
        *self == Meta::Alive
    }

    fn is_dead(&self) -> bool {
        *self == Meta::Dead
    }
}

impl<T> Default for Store<T> {
    fn default() -> Self {
        Self {
            keys: vec![],
            values: vec![],
            meta: vec![],
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

impl<T> Store<T> {
    pub fn new() -> Self {
        Default::default()
    }

    /// Compact all tombstones after a given index.
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
        // committing inserts handles compaction, and tries to reuse tombstones.
        self.commit_pending_inserts();
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
            Err(_) if self.pending_inserts.contains_key(id) => SearchResult::Pending,
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
                Some(old)
            }
            SearchResult::TombstoneAvailable(i) => {
                self.meta[i] = Meta::Alive;
                self.keys[i] = *id;
                self.values[i] = val;
                None
            }
            SearchResult::InsertBefore(i) => {
                // If i is at the end of the vector, fast case it.
                if i == self.keys.len() {
                    self.keys.push(*id);
                    self.values.push(val);
                    self.meta.push(Meta::Alive);
                    return None;
                }
                self.pending_inserts.insert(*id, val)
            }
            SearchResult::Pending => self.pending_inserts.insert(*id, val),
        }
    }

    pub fn commit_pending_inserts(&mut self) {
        self.pending_inserts.retain(|id, val| {
            let res = self.keys.binary_search(id);
            match res {
                Err(i) if i < self.meta.len() && self.meta[i].is_dead() => {
                    self.keys[i] = *id;
                    // val is `&mut T`; we need to steal it because Rust doesn't understand that we're going to return
                    // false and drop it from the vec.
                    std::mem::swap(&mut self.values[i], val);
                    self.meta[i] = Meta::Alive;
                    false
                }
                _ => true,
            }
        });

        // Now that we've reused what tombstones we can, get rid of the rest.
        self.compact();

        let mut iterator = std::mem::take(&mut self.pending_inserts).into_iter();

        // While we're not just pushing to the end, do some looping.
        for (k, v) in &mut iterator {
            // The fast and common case is that we're just pushing to the end. Break out once we detect this.
            if let Some(l) = self.keys.last().cloned() {
                if k > l {
                    // k is consumed, we must deal with it here because we can't put it back on the iterator.
                    self.keys.push(k);
                    self.values.push(v);
                    self.meta.push(Meta::Alive);
                    break;
                }
            }

            let ind = match self.keys.binary_search(&k) {
                Ok(_) => panic!("We already did all the inserts of items in the map and shouldn't find another we already had"),
                Err(x) => x,
            };
            self.keys.insert(ind, k);
            self.values.insert(ind, v);
            // Compacting already cleared the tombstones; we need only make sure it stays the right size.
            self.meta.push(Meta::Alive);
        }

        // Most of the work actually happens here: the special cases above were just getting things out of the way.
        // Object id generationguarantees ordering per run, so the only times we really see the above cases are at
        // startup if the clock went backward or when loading from saved data.
        for (k, v) in iterator {
            self.keys.push(k);
            self.values.push(v);
            self.meta.push(Meta::Alive);
        }

        assert_eq!(self.keys.len(), self.values.len());
        assert_eq!(self.values.len(), self.meta.len());
    }

    pub fn get_by_id(&self, id: &ObjectId) -> Option<&T> {
        match self.search_index_from_id(id) {
            SearchResult::Found(i) => self.values.get(i),
            SearchResult::TombstoneAvailable(_) | SearchResult::InsertBefore(_) => None,
            SearchResult::Pending => self.pending_inserts.get(id),
        }
    }

    pub fn get_by_id_mut(&mut self, id: &ObjectId) -> Option<&mut T> {
        match self.search_index_from_id(id) {
            SearchResult::Found(i) => self.values.get_mut(i),
            SearchResult::TombstoneAvailable(_) | SearchResult::InsertBefore(_) => None,
            SearchResult::Pending => self.pending_inserts.get_mut(id),
        }
    }

    pub fn get_by_index(&self, index: usize) -> Option<&T> {
        self.values.get(index)
    }

    pub fn get_by_index_mut(&mut self, index: usize) -> Option<&mut T> {
        self.values.get_mut(index)
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
    pub fn iter(&self) -> impl Iterator<Item = (usize, ObjectId, &T)> {
        self.keys
            .iter()
            .enumerate()
            .zip(self.values.iter())
            .filter_map(move |((i, k), v)| {
                if self.meta[i].is_dead() {
                    return None;
                }
                Some((i, *k, v))
            })
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (usize, ObjectId, &mut T)> {
        let ks = &self.keys;
        let vs = &mut self.values;
        let ms = &self.meta;
        ks.iter()
            .enumerate()
            .zip(vs.iter_mut())
            .filter_map(move |((i, k), v)| {
                if ms[i].is_dead() {
                    return None;
                }
                Some((i, *k, v))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_ordered_inserting() {
        let mut store: Store<u64> = Store::new();
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
            assert_eq!(store.get_by_index(i).unwrap(), &((i + 1) as u64));
        }
    }

    #[test]
    fn insert_reusing_tombstones() {
        let mut store: Store<u64> = Store::new();
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
        assert_eq!(store.get_by_index(2).unwrap(), &5);

        store.delete_index(1);
        store.delete_index(3);
        assert!(!store.is_index_alive(1));
        assert!(!store.is_index_alive(3));

        store.insert(&ObjectId::new_testing(3), 3);
        store.insert(&ObjectId::new_testing(7), 7);
        assert_eq!(store.id_at_index(1), ObjectId::new_testing(3));
        assert_eq!(store.get_by_index(1).unwrap(), &3);
        assert!(store.is_index_alive(1));
        assert_eq!(store.id_at_index(3), ObjectId::new_testing(7));
        assert_eq!(store.get_by_index(3).unwrap(), &7);
        assert!(store.is_index_alive(3));

        // Reuse of the first and last elements are an interesting case worth doing.
        store.delete_index(0);
        assert!(!store.is_index_alive(0));
        store.insert(&ObjectId::new_testing(1), 1);
        assert!(store.is_index_alive(0));
        assert_eq!(store.get_by_index(0).unwrap(), &1);
        assert_eq!(store.id_at_index(0), ObjectId::new_testing(1));

        store.delete_index(4);
        assert!(!store.is_index_alive(4));
        // We can use any object id here.
        store.insert(&ObjectId::new_testing(100), 100);
        assert!(store.is_index_alive(4));
        assert_eq!(store.id_at_index(4), ObjectId::new_testing(100));
        assert_eq!(store.get_by_index(4).unwrap(), &100);
    }

    #[test]
    fn test_pending_inserts() {
        let mut store: Store<u64> = Store::new();

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
        assert_eq!(&store.meta, &vec![Meta::Alive; 5]);
    }

    #[test]
    fn test_mutation() {
        let mut store: Store<u64> = Store::new();

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
                .map(|x| (*x.0, *x.1))
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
        assert_eq!(&store.meta, &vec![Meta::Alive; 5],);
    }

    #[test]
    fn test_get_by_id() {
        let mut store: Store<u64> = Store::new();
        // Leave some holes.
        for i in 1..=10 {
            store.insert(&ObjectId::new_testing(i * 2), i * 2);
        }

        for i in 1..=10 {
            assert_eq!(
                store.get_by_id(&ObjectId::new_testing(i * 2)),
                Some(&(i * 2))
            );
            assert_eq!(
                store.get_by_id_mut(&ObjectId::new_testing(i * 2)),
                Some(&mut (i * 2))
            );

            // Check that any index we shouldn't have isn't present.
            assert_eq!(store.get_by_id(&ObjectId::new_testing(i * 2 + 1)), None);
            assert_eq!(store.get_by_id_mut(&ObjectId::new_testing(i * 2 + 1)), None);
        }

        // Putting something in the pending inserts should work.
        store.insert(&ObjectId::new_testing(1), 1);
        assert_eq!(store.get_by_id(&ObjectId::new_testing(1)), Some(&1));
        assert_eq!(store.get_by_id_mut(&ObjectId::new_testing(1)), Some(&mut 1));
    }

    #[test]
    fn test_compaction() {
        let mut store: Store<u64> = Store::new();
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
            &vec![Meta::Alive, Meta::Dead, Meta::Alive, Meta::Dead, Meta::Dead]
        );
    }

    #[test]
    fn test_iter() {
        let mut store: Store<u64> = Store::new();

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
        let mut store: Store<u64> = Store::new();

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
}
