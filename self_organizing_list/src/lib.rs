//?! A self-organizing list to accelerate linear searches when caching.
//!
//! See [Wikipedia](https://en.wikipedia.org/wiki/Self-organizing_list) for an
//! overview.  We implement transpose.
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
fn transpose<T: Copy>(slice: &mut [T], index: usize) {
    if index == 0 {
        return;
    }

    debug_assert!(index < slice.len());
    unsafe {
        let e1 = *slice.get_unchecked(index - 1);
        let e2 = *slice.get_unchecked(index);
        *slice.get_unchecked_mut(index - 1) = e2;
        *slice.get_unchecked_mut(index) = e1;
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
            let len = slice.len();
            for i in 0..len {
                if &slice.get_unchecked(i).key == key {
                    let ret = Some(slice.get_unchecked(i).value);
                    transpose(slice, i);
                    return ret;
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn get_internal_vec<const SIZE: usize>(
        l: &SelfOrganizingList<usize, usize, SIZE>,
    ) -> Vec<(usize, usize)> {
        unsafe {
            (*l.entries.get())
                .iter()
                .map(|x| (x.key, x.value))
                .collect()
        }
    }

    #[test]
    fn test_add_to_cache() {
        let l = SelfOrganizingList::<usize, usize, 5>::new();
        l.add_to_cache(1, 2);
        l.add_to_cache(2, 4);
        l.add_to_cache(3, 6);
        l.add_to_cache(4, 8);
        l.add_to_cache(5, 10);
        assert_eq!(
            get_internal_vec(&l),
            vec![(1, 2), (2, 4), (3, 6), (4, 8), (5, 10)]
        );
        l.add_to_cache(11, 22);
        assert_eq!(
            get_internal_vec(&l),
            vec![(1, 2), (2, 4), (3, 6), (4, 8), (11, 22)]
        );
    }

    #[test]
    fn test_transposing() {
        let l = SelfOrganizingList::<usize, usize, 5>::new();
        l.add_to_cache(1, 2);
        l.add_to_cache(2, 4);
        l.add_to_cache(3, 6);
        l.add_to_cache(4, 8);
        l.add_to_cache(5, 10);
        assert_eq!(
            get_internal_vec(&l),
            vec![(1, 2), (2, 4), (3, 6), (4, 8), (5, 10)]
        );
        assert_eq!(l.read_cache(&3), Some(6));
        assert_eq!(
            get_internal_vec(&l),
            vec![(1, 2), (3, 6), (2, 4), (4, 8), (5, 10)]
        );
        assert_eq!(l.read_cache(&5), Some(10));
        assert_eq!(
            get_internal_vec(&l),
            vec![(1, 2), (3, 6), (2, 4), (5, 10), (4, 8)]
        );
    }
}
