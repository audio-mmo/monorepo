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
    TombstoneAvailabel(usize),
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
            Ok(i) | Err(i) if self.tombstones[i] => SearchResult::TombstoneAvailabel(i),
            Err(_) if self.pending_inserts.contains_key(id) => SearchResult::Pending,
            Ok(i) => SearchResult::Found(i),
            Err(i) => SearchResult::InsertBefore(i),
        }
    }

    fn insert(&mut self, id: &ObjectId, val: T) -> Option<T> {
        match self.search_index_from_id(id) {
            SearchResult::Found(i) => {
                let mut old = val;
                std::mem::swap(&mut self.values[i], &mut old);
                Some(old)
            }
            SearchResult::TombstoneAvailabel(i) => {
                self.tombstones.set(i, false);
                self.keys[i] = *id;
                self.values[i] = val;
                None
            }
            SearchResult::InsertBefore(_) | SearchResult::Pending => {
                self.pending_inserts.insert(*id, val)
            }
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
                Err(i) if self.tombstones[i] => {
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
            SearchResult::TombstoneAvailabel(_) | SearchResult::InsertBefore(_) => None,
            SearchResult::Pending => self.pending_inserts.get(id),
        }
    }

    fn get_by_id_mut(&mut self, id: &ObjectId) -> Option<&mut T> {
        match self.search_index_from_id(id) {
            SearchResult::Found(i) => self.values.get_mut(i),
            SearchResult::TombstoneAvailabel(_) | SearchResult::InsertBefore(_) => None,
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
            SearchResult::InsertBefore(_) | SearchResult::TombstoneAvailabel(_) => false,
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
