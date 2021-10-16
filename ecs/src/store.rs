#![allow(dead_code)]
//! Implements a vector-backed storage keyed by `ObjectId` intended for use with components.
//!
//! This isn't limited to components because it can store anything.
use std::cell::UnsafeCell;
use std::collections::BTreeMap;

use bitvec::vec::BitVec;

use crate::object_id::ObjectId;

/// A `Store` is like a combinationb between an array and a map: it is possible to ask about specific indices, as well
/// as specific keys.
///
/// Internally, this is a set of 3 arrays: an array of object id keys, an array of values, and an array of tombstones.
/// The tombstones are toggled on when objects are deleted rather than moving objects so that it is possible to iterate
/// over the store while deleting.  The map isn't threadsafe and uses interior mutability, so that it is possible to
/// iterate over it while performing mutable operations.  Most operations panic if called in a way which isn't safe,
/// e.g. using indices past the end.
///
/// `maintenance` should periodically be called in order to clear out tombstones and actually drop values.  Otherwise
/// the map never reclaims space.  Until `maintenance` is called, indices are stable on deletion.
///
/// Indices are stable on insertion, but newly inserted items may not be visible until `commit_inserts` or `maintenance`
/// is called.  Inserts which replace a value for an item already in the store will always be immediately visible.
///
/// Iteration is always in order by increasing id, which is the primary invariant we wish to maintain: it allows for
/// efficient merges.  This is why the other invariants and usage patterns are slightly awkward: without compromises,
/// this can only be done with very frequent sorting.
///
/// This container is highly optimized for mostly sorted data.
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

    fn get_by_id(&mut self, id: &ObjectId) -> Option<&T> {
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
    fn commit_inserts(&mut self) {}
}
