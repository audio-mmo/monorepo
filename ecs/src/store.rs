//! The [Store] is a store optimized for in-order object ids, using 3 vecs and a BTreeMap.  See the comments on the
//! [Store] type for details.
use std::collections::BTreeMap;

use bitvec::vec::BitVec;

use crate::object_id::ObjectId;

/// A store for component data consisting of some vecs.
///
/// This implementation uses:
///
/// - A vec of keys, which must be [ObjectId]s.
/// - A vec of values, which can be anything.
/// - A bitvec of tombstones, which are toggled to true when objects are deleted.
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
///
/// It is possible to iterate using a [StoreVisitor] which allows you to modify/delete objects from the store while
/// visiting it, and the visitor will handle this case by figuring out what the next-largest id from the one it last saw
/// was.  Deleting an object which you haven't seen yet will observe the delete.  Iterating over the store (via the
/// visitor or via the iteration api) only iterates over committed inserts, and iterates in order of increasing object
/// id.
///
/// The design here is optimized for the case in which we want to join multiple stores to perform queries in `O(1)`
/// additional memory and `O(n)` time.  Basically, when deleting/inserting in higher level components, insert/deletions
/// may not be visible until the next tick, but modifications are visible immediately.  The interior mutability avoids
/// issues like needing to build up lists of changes, by allowing modification while iterating: we assume that memory is
/// the most expensive component and take the CPU hit to make that happen.
///
/// Methods which don't return `Option` use normal `[]` indexing under the hood and generally panic on invariant
/// failures.
pub struct Store<T> {
    keys: Vec<ObjectId>,
    values: Vec<T>,
    tombstones: BitVec,
    pending_inserts: BTreeMap<ObjectId, T>,
}

impl<T> Default for Store<T> {
    fn default() -> Self {
        Self {
            keys: vec![],
            values: vec![],
            tombstones: BitVec::new(),
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
        let keys = &mut self.keys;
        let tombs = &mut self.tombstones;
        keys.retain(|_| {
            let ret = !tombs[key_ind];
            key_ind += 1;
            ret
        });

        let mut val_ind = 0;
        let vals = &mut self.values;
        vals.retain(|_| {
            let ret = !tombs[val_ind];
            val_ind += 1;
            ret
        });

        self.tombstones.truncate(self.keys.len());
        self.tombstones.set_elements(0);
    }

    pub fn maintenance(&mut self) {
        // committing inserts handles compaction, and tries to reuse tombstones.
        self.commit_pending_inserts();
        self.keys.shrink_to_fit();
        self.values.shrink_to_fit();
        self.tombstones.shrink_to_fit();
    }

    /// Public wrapper of binary_search from std.
    fn binary_search(&self, id: &ObjectId) -> Result<usize, usize> {
        self.keys.binary_search(id)
    }

    fn search_index_from_id(&self, id: &ObjectId) -> SearchResult {
        let ind = self.keys.binary_search(id);
        match ind {
            Ok(i) | Err(i) if i < self.tombstones.len() && self.tombstones[i] => {
                SearchResult::TombstoneAvailable(i)
            }
            Err(_) if self.pending_inserts.contains_key(id) => SearchResult::Pending,
            Ok(i) => SearchResult::Found(i),
            Err(i) => {
                // Special case: if the end of the vector is a tombstone, use that.
                if let Some(t) = self.tombstones.last() {
                    if *t {
                        return SearchResult::TombstoneAvailable(self.tombstones.len() - 1);
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
                self.tombstones.set(i, false);
                self.keys[i] = *id;
                self.values[i] = val;
                None
            }
            SearchResult::InsertBefore(i) => {
                // If i is at the end of the vector, fast case it.
                if i == self.keys.len() {
                    self.keys.push(*id);
                    self.values.push(val);
                    self.tombstones.push(false);
                    return None;
                }
                self.pending_inserts.insert(*id, val)
            }
            SearchResult::Pending => self.pending_inserts.insert(*id, val),
        }
    }

    pub fn commit_pending_inserts(&mut self) {
        // First, insert anything we can insert via a tombstone.  We have to help the borrow checker out here until
        // edition 2021 is stable.
        let mut pi = BTreeMap::new();
        std::mem::swap(&mut pi, &mut self.pending_inserts);

        pi.retain(|id, val| {
            let res = self.keys.binary_search(id);
            match res {
                Err(i) if i < self.tombstones.len() && self.tombstones[i] => {
                    self.keys[i] = *id;
                    std::mem::swap(&mut self.values[i], val);
                    self.tombstones.set(i, false);
                    false
                }
                _ => true,
            }
        });

        // Now that we've reused what tombstones we can, get rid of the rest.
        self.compact();

        let mut iterator = pi.into_iter();

        // While we're not just pushing to the end, do some looping.
        for (k, v) in &mut iterator {
            // The fast and common case is that we're just pushing to the end. Break out once we detect this.
            if let Some(l) = self.keys.last().cloned() {
                if k > l {
                    // k is consumed, we must deal with it.
                    self.keys.push(k);
                    self.values.push(v);
                    self.tombstones.push(false);
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
            self.tombstones.push(false);
        }

        // Most of the work actually happens here: the special cases above were just getting things out of the way.
        // Object id generationguarantees ordering per run, so the only times we really see the above cases are at
        // startup if the clock went backward or when loading from saved data.
        for (k, v) in iterator {
            self.keys.push(k);
            self.values.push(v);
            self.tombstones.push(false);
        }

        assert_eq!(self.keys.len(), self.values.len());
        assert_eq!(self.values.len(), self.tombstones.len());
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
                self.tombstones.set(i, true);
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
        let ret = self.tombstones[index];
        self.tombstones.set(index, true);
        // Ret is true if there was already a tombstone, e.g. we did nothing.
        !ret
    }

    pub fn is_index_alive(&self, index: usize) -> bool {
        !self.tombstones[index]
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
                if self.tombstones[i] {
                    return None;
                }
                Some((i, *k, v))
            })
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (usize, ObjectId, &mut T)> {
        let ks = &self.keys;
        let vs = &mut self.values;
        let ts = &self.tombstones;
        ks.iter()
            .enumerate()
            .zip(vs.iter_mut())
            .filter_map(move |((i, k), v)| {
                if ts[i] {
                    return None;
                }
                Some((i, *k, v))
            })
    }
}

pub struct StoreVisitor<T> {
    last_seen_id: Option<ObjectId>,
    last_index: usize,
    pd: std::marker::PhantomData<*const T>,
}

impl<T> StoreVisitor<T> {
    pub fn new(#[allow(unused_variables)] store: &Store<T>) -> StoreVisitor<T> {
        StoreVisitor {
            last_index: 0,
            last_seen_id: None,
            pd: Default::default(),
        }
    }

    fn advance_past_tombstones(&mut self, store: &Store<T>) {
        while self.last_index < store.index_len() && !store.is_index_alive(self.last_index) {
            self.last_index += 1;
        }
    }

    /// Advance the index.
    ///
    /// This deals with object deletion, as well as index shifts: we compare against the last object id, and binary
    /// search to find the first thing after it when we detect a mismatch.
    fn incr_index(&mut self, store: &Store<T>) {
        if self.last_index >= store.index_len() {
            return;
        }

        // if we have a last seen id, sanity check it, then bump by one.
        if let Some(ref oid) = self.last_seen_id {
            if store.id_at_index(self.last_index) == *oid {
                // Our index is good. just increment it.
                self.last_index += 1;
            }
            // Otherwise, we need to find the nearest index to the object and base it off that.
            self.last_index = match store.binary_search(oid) {
                Ok(i) => i + 1,
                // We didn't find it, but we have the index to insert before; this is our next index.
                Err(i) => i,
            };
        }
        // Otherwise we haven't advanced yet.

        self.advance_past_tombstones(store);

        // Our last seen id is the index we're currently at, if this is possible to get.
        if self.last_index < store.index_len() {
            self.last_seen_id = Some(store.id_at_index(self.last_index));
        }
    }

    pub fn next<'a>(&mut self, store: &'a Store<T>) -> Option<(usize, ObjectId, &'a T)> {
        self.incr_index(store);
        if self.last_index >= store.index_len() {
            return None;
        }

        return Some((
            self.last_index,
            store.id_at_index(self.last_index),
            store.get_by_index(self.last_index).unwrap(),
        ));
    }

    pub fn next_mut<'a>(
        &mut self,
        store: &'a mut Store<T>,
    ) -> Option<(usize, ObjectId, &'a mut T)> {
        self.incr_index(store);
        if self.last_index >= store.index_len() {
            return None;
        }

        Some((
            self.last_index,
            store.id_at_index(self.last_index),
            store.get_by_index_mut(self.last_index).unwrap(),
        ))
    }

    /// Peak at the next id this visitor will return, assuming that the store isn't modified in the meantime.
    ///
    /// Used in the query infrastructure.
    pub fn peak_id(&mut self, store: &Store<T>) -> Option<ObjectId> {
        let old_last_index = self.last_index;
        let old_objid = self.last_seen_id;
        let ret = self.next(store).map(|x| x.1);
        self.last_index = old_last_index;
        self.last_seen_id = old_objid;
        ret
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use bitvec::prelude::*;

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
        assert_eq!(&store.tombstones, &bitvec::bitvec![0; 5]);
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
        assert_eq!(&store.tombstones, &bitvec::bitvec![0; 5],);
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
        assert_eq!(&store.tombstones, &bitvec::bitvec![0, 1, 0, 1, 1]);
    }

    #[test]
    fn test_visitor_basic() {
        let mut store: Store<u64> = Store::new();
        for i in 1..=5 {
            store.insert(&ObjectId::new_testing(i), i);
        }

        let mut vis = StoreVisitor::new(&store);
        let mut res = vec![];
        while let Some((_, k, v)) = vis.next(&store) {
            res.push((k.get_counter(), *v));
        }

        assert_eq!(res, vec![(1, 1), (2, 2), (3, 3), (4, 4), (5, 5)]);
    }

    #[test]
    fn test_visitor_tombstone() {
        let mut store: Store<u64> = Store::new();
        for i in 1..=10 {
            store.insert(&ObjectId::new_testing(i), i);
        }

        store.delete_index(1);
        store.delete_index(3);
        store.delete_index(4);
        store.delete_index(9);

        let mut vis = StoreVisitor::new(&store);
        let mut res = vec![];
        while let Some((_, k, v)) = vis.next(&store) {
            res.push((k.get_counter(), *v));
        }

        assert_eq!(res, vec![(1, 1), (3, 3), (6, 6), (7, 7), (8, 8), (9, 9)]);
    }

    #[test]
    fn test_visitor_tombstone_with_delete_before() {
        let mut store: Store<u64> = Store::new();
        for i in 1..=10 {
            store.insert(&ObjectId::new_testing(i), i);
        }

        let mut vis = StoreVisitor::new(&store);
        let mut res = vec![];
        for _ in 0..3 {
            res.push(vis.next(&store).map(|x| (x.1.get_counter(), *x.2)).unwrap());
        }

        store.delete_index(0);
        store.maintenance();

        while let Some((_, k, v)) = vis.next(&store) {
            res.push((k.get_counter(), *v));
        }

        assert_eq!(
            res,
            vec![
                (1, 1),
                (2, 2),
                (3, 3),
                (4, 4),
                (5, 5),
                (6, 6),
                (7, 7),
                (8, 8),
                (9, 9),
                (10, 10),
            ]
        );
    }

    #[test]
    fn test_visitor_on_empty_store() {
        let store: Store<u64> = Store::new();
        let mut vis = StoreVisitor::new(&store);
        assert_eq!(vis.next(&store), None);
    }

    #[test]
    fn test_visitor_peak() {
        let mut store: Store<u64> = Store::new();
        for i in 1..=5 {
            store.insert(&ObjectId::new_testing(i), i);
        }

        let mut vis = StoreVisitor::new(&store);
        let mut res = vec![];
        let mut peak = vec![vis.peak_id(&store)];
        while let Some((_, k, v)) = vis.next(&store) {
            res.push((k.get_counter(), *v));
            peak.push(vis.peak_id(&store));
        }
        let peak = peak
            .into_iter()
            .map(|x| x.map(|y| y.get_counter()))
            .collect::<Vec<_>>();

        assert_eq!(res, vec![(1, 1), (2, 2), (3, 3), (4, 4), (5, 5)]);
        assert_eq!(
            peak,
            vec![Some(1), Some(2), Some(3), Some(4), Some(5), None]
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
