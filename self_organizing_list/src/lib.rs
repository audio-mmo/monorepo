//?! A self-organizing list to accelerate linear searches when caching.
//!
//! See [Wikipedia](https://en.wikipedia.org/wiki/Self-organizing_list) for an
//! overview.  We implement move-to-front.
//!
//! Note that the primary indirect consumer is `ChunkedGrid`, and many
//! assumptions here are made with that in mind.
use std::cell::UnsafeCell;

use arrayvec::ArrayVec;

#[derive(Copy, Clone, Debug)]
struct Entry<K, V> {
    key: K,
    value: V,
}

#[derive(Debug)]
pub struct SelfOrganizingList<K, V, const ENTRIES: usize> {
    entries: UnsafeCell<ArrayVec<Entry<K, V>, ENTRIES>>,
}

#[inline(always)]
fn move_to_front<T: Copy>(slice: &mut [T], index: usize) {
    if index == 0 {
        return;
    }

    unsafe {
        debug_assert!(index < slice.len());
        let new_front = *slice.get_unchecked(index);
        for i in 1..index {
            *slice.get_unchecked_mut(i) = *slice.get_unchecked(i - 1);
        }
        *slice.get_unchecked_mut(0) = new_front;
    }
}

impl<K: Copy + Eq, V: Copy + Eq, const ENTRIES: usize> SelfOrganizingList<K, V, ENTRIES> {
    pub fn new() -> SelfOrganizingList<K, V, ENTRIES> {
        SelfOrganizingList {
            entries: Default::default(),
        }
    }

    #[inline]
    pub fn read_cache(&self, key: &K) -> Option<V> {
        unsafe {
            let cur_cache = self.entries.get();
            let slice = &mut (*cur_cache)[..];
            for i in 0..slice.len() {
                if &slice.get_unchecked(i).key != key {
                    continue;
                }
                move_to_front(slice, i);
                return Some(slice.get_unchecked(0).value);
            }
        }
        None
    }

    #[inline]
    fn add_to_cache(&self, key: K, value: V) {
        unsafe {
            let cache = self.entries.get();
            let ent = Entry { key, value };
            if (*cache).try_push(ent).is_ok() {
                return;
            }
            *(*cache).get_unchecked_mut((*cache).len() - 1) = ent;
        }
    }

    /// Either read the cache or insert into it.  The specified callback may choose not to insert by returning `None`.
    #[inline(always)]
    pub fn read_or_insert(
        &mut self,
        key: &K,
        mut callback: impl FnMut(&K) -> Option<V>,
    ) -> Option<V> {
        if let Some(v) = self.read_cache(key) {
            return Some(v);
        }
        let nv = callback(key)?;
        self.add_to_cache(*key, nv);
        Some(nv)
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        unsafe { (*self.entries.get()).clear() }
    }
}

impl<K: Copy + Eq, V: Copy + Eq, const ENTRIES: usize> Default
    for SelfOrganizingList<K, V, ENTRIES>
{
    fn default() -> Self {
        Self::new()
    }
}
