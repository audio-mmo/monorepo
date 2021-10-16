#![allow(dead_code)]
use std::cell::UnsafeCell;
use std::collections::BTreeMap;

use bitvec::vec::BitVec;

use crate::object_id::ObjectId;

pub struct Store<T> {
    state: UnsafeCell<StoreState<T>>,
}

struct StoreState<T> {
    keys: Vec<ObjectId>,
    values: Vec<T>,
    tombstones: BitVec,
    pending_inserts: BTreeMap<ObjectId, T>,
}

impl<T> Default for StoreState<T> {
    fn default() -> Self {
        Self {
            keys: vec![],
            values: vec![],
            tombstones: BitVec::new(),
            pending_inserts: Default::default(),
        }
    }
}

impl<T> Default for Store<T> {
    fn default() -> Self {
        Store {
            state: UnsafeCell::new(Default::default()),
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

impl<T> StoreState<T> {
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

    fn maintenance(&mut self) {
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

    fn insert(&mut self, id: &ObjectId, val: T) -> Option<T> {
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

    fn commit_pending_inserts(&mut self) {
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

    fn get_by_id(&self, id: &ObjectId) -> Option<&T> {
        match self.search_index_from_id(id) {
            SearchResult::Found(i) => self.values.get(i),
            SearchResult::TombstoneAvailable(_) | SearchResult::InsertBefore(_) => None,
            SearchResult::Pending => self.pending_inserts.get(id),
        }
    }

    fn get_by_id_mut(&mut self, id: &ObjectId) -> Option<&mut T> {
        match self.search_index_from_id(id) {
            SearchResult::Found(i) => self.values.get_mut(i),
            SearchResult::TombstoneAvailable(_) | SearchResult::InsertBefore(_) => None,
            SearchResult::Pending => self.pending_inserts.get_mut(id),
        }
    }

    fn get_by_index(&self, index: usize) -> Option<&T> {
        self.values.get(index)
    }

    fn get_by_index_mut(&mut self, index: usize) -> Option<&mut T> {
        self.values.get_mut(index)
    }

    fn index_for_id(&self, id: &ObjectId) -> Option<usize> {
        match self.search_index_from_id(id) {
            SearchResult::Found(i) => Some(i),
            _ => None,
        }
    }

    fn delete_id(&mut self, id: &ObjectId) -> bool {
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

    fn delete_index(&mut self, index: usize) -> bool {
        let ret = self.tombstones[index];
        self.tombstones.set(index, true);
        // Ret is true if there was already a tombstone, e.g. we did nothing.
        !ret
    }

    fn is_index_alive(&self, index: usize) -> bool {
        !self.tombstones[index]
    }

    fn index_len(&self) -> usize {
        self.keys.len()
    }

    fn id_at_index(&self, index: usize) -> ObjectId {
        self.keys[index]
    }
}

impl<T> Store<T> {
    pub fn new() -> Store<T> {
        Default::default()
    }

    /// Call a function on the state.  Hides the `UnsafeCell` behind a safe interface.
    ///
    /// Since this isn't sync, we know that only one caller can be in here at once, and that thus our invariant is that
    /// we must be in a consistent state when we leave this function.
    fn with_state<R>(&self, cb: impl FnOnce(&mut StoreState<T>) -> R) -> R {
        unsafe {
            let ptr = self.state.get();
            cb(&mut *ptr)
        }
    }

    /// Commit a batch of inserts.
    fn commit_pending_inserts(&mut self) {
        self.with_state(|s| s.commit_pending_inserts())
    }

    /// Perform maintenance. Commits inserts which are outstanding and shrinks the internal arrays to reclaim space.
    fn maintenance(&mut self) {
        self.with_state(|s| s.maintenance())
    }

    fn insert(&self, key: &ObjectId, val: T) -> Option<T> {
        self.with_state(|s| s.insert(key, val))
    }

    pub fn get_by_id(&self, id: &ObjectId) -> Option<&T> {
        // We use the fact that only &mut methods move things around to prove safety.
        unsafe { (&*self.state.get()).get_by_id(id) }
    }

    pub fn get_by_id_mut(&mut self, id: &ObjectId) -> Option<&mut T> {
        unsafe { (&mut *self.state.get()).get_by_id_mut(id) }
    }

    // Read a particular index. Don't check the tombstone.
    pub fn get_by_index(&self, index: usize) -> Option<&T> {
        unsafe { (&*self.state.get()).get_by_index(index) }
    }

    /// Read a particular index. Don't check the tombstone.
    pub fn get_by_index_mut(&mut self, index: usize) -> Option<&mut T> {
        unsafe { (&mut *self.state.get()).get_by_index_mut(index) }
    }

    /// Is the specified index alive?
    pub fn is_index_alive(&self, index: usize) -> bool {
        self.with_state(|s| s.is_index_alive(index))
    }

    /// Find the index for a particular object id. Returns `Some(index)` if the object is in the store and is a
    /// committed insert.
    pub fn index_for_id(&self, id: &ObjectId) -> Option<usize> {
        self.with_state(|s| s.index_for_id(id))
    }

    /// Return the length of the index portion.
    pub fn index_len(&self) -> usize {
        self.with_state(|s| s.index_len())
    }

    fn id_at_index(&self, index: usize) -> ObjectId {
        self.with_state(|s| s.id_at_index(index))
    }

    fn binary_search(&self, id: &ObjectId) -> Result<usize, usize> {
        self.with_state(|s| s.binary_search(id))
    }

    fn delete_id(&mut self, id: &ObjectId) -> bool {
        self.with_state(|s| s.delete_id(id))
    }

    fn delete_index(&mut self, index: usize) -> bool {
        self.with_state(|s| s.delete_index(index))
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
        while !store.is_index_alive(self.last_index) {
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

    pub fn next<'a>(&mut self, store: &'a Store<T>) -> Option<(ObjectId, &'a T)> {
        self.incr_index(store);
        if self.last_index >= store.index_len() {
            return None;
        }

        return Some((
            store.id_at_index(self.last_index),
            store.get_by_index(self.last_index).unwrap(),
        ));
    }

    pub fn next_mut<'a>(&mut self, store: &'a mut Store<T>) -> Option<(ObjectId, &'a mut T)> {
        self.incr_index(store);
        if self.last_index >= store.index_len() {
            return None;
        }

        Some((
            store.id_at_index(self.last_index),
            store.get_by_index_mut(self.last_index).unwrap(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use bitvec::prelude::*;

    #[test]
    fn basic_ordered_inserting() {
        let store: Store<u64> = Store::new();
        for i in 1..=5 {
            store.insert(&ObjectId::new_testing(i), i);
        }

        // The index portion should contain everything.
        assert_eq!(store.index_len(), 5);
        assert_eq!(unsafe { (*store.state.get()).pending_inserts.len() }, 0);

        assert_eq!(
            unsafe { &(*store.state.get()).keys },
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
        assert_eq!(unsafe { (*store.state.get()).pending_inserts.len() }, 3);
        store.commit_pending_inserts();

        assert_eq!(
            unsafe { &(*store.state.get()).keys },
            &vec![
                ObjectId::new_testing(1),
                ObjectId::new_testing(2),
                ObjectId::new_testing(3),
                ObjectId::new_testing(4),
                ObjectId::new_testing(5)
            ]
        );
        assert_eq!(
            unsafe { &(*store.state.get()).values },
            &vec![1, 2, 3, 4, 5]
        );
        assert_eq!(
            unsafe { &(*store.state.get()).tombstones },
            &bitvec::bitvec![0; 5]
        );
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
            unsafe { &(*store.state.get()).keys },
            &vec![ObjectId::new_testing(1), ObjectId::new_testing(5)]
        );
        assert_eq!(unsafe { &(*store.state.get()).values }, &vec![11, 15]);
        assert_eq!(
            unsafe { &(*store.state.get()).pending_inserts }
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
            unsafe { &(*store.state.get()).keys },
            &vec![
                ObjectId::new_testing(1),
                ObjectId::new_testing(2),
                ObjectId::new_testing(3),
                ObjectId::new_testing(4),
                ObjectId::new_testing(5)
            ]
        );
        assert_eq!(
            unsafe { &(*store.state.get()).values },
            &vec![11, 12, 13, 14, 15]
        );
        assert_eq!(
            unsafe { &(*store.state.get()).tombstones },
            &bitvec::bitvec![0; 5],
        );
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
            unsafe { &(*store.state.get()).keys },
            &vec![
                ObjectId::new_testing(1),
                ObjectId::new_testing(2),
                ObjectId::new_testing(3),
                ObjectId::new_testing(4),
                ObjectId::new_testing(5)
            ]
        );
        assert_eq!(
            unsafe { &(*store.state.get()).values },
            &vec![1, 2, 3, 4, 5]
        );
        assert_eq!(
            unsafe { &(*store.state.get()).tombstones },
            &bitvec::bitvec![0, 1, 0, 1, 1]
        );
    }
}
