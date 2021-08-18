//! A [CachedHashMap] is a [HashMap] replacement which defers to an
//! internal cache before reading the key.
//!
//! So long as the hashmap is immutably borrowed, the cache remains valid and
//! uses a bit of unsafe iun order to return direct pointers to the entries.
//! When a mutable borrow is taken, the cache is invalidated until the next time
//! there are only immutable references.  
//!
//! Finally, this hashmap uses a non-cryptographic hash because it is already vulnerable to DOS:
//! reading different items is much slower than reading the same item.
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::ops::{Deref, DerefMut};

use ammo_self_organizing_list::*;

// make the hasher easy to change by renaming it. Otherwise, we have to change
// it in lots of places if we change the hashing algorithm.
type Hasher = std::hash::BuildHasherDefault<rustc_hash::FxHasher>;

/// Currently defaults for generic parameters are experimental so we must hardcode.
const CACHE_SIZE: usize = 3;

pub struct CachedHashMap<K, V> {
    inner: HashMap<K, V, Hasher>,
    cache: UnsafeCell<SelfOrganizingList<K, *const V, CACHE_SIZE>>,
}

pub struct CachedBorrowMut<'a, K, V> {
    reference: &'a mut CachedHashMap<K, V>,
}

impl<K: Eq + Copy + Hash, V> CachedHashMap<K, V> {
    pub fn new() -> CachedHashMap<K, V> {
        CachedHashMap {
            inner: Default::default(),
            cache: UnsafeCell::new(Default::default()),
        }
    }

    fn get_cached_ptr(&self, key: &K) -> Option<*const V> {
        unsafe {
            let cache = self.cache.get();
            (*cache).read_or_insert(key, |_| {
                let r = self.inner.get(key)?;
                Some(r as *const V)
            })
        }
    }

    /// Override of [HashMap::get] which tries the cache first.
    #[inline]
    pub fn get_cached(&self, key: &K) -> Option<&V> {
        Some(unsafe { &*(self.get_cached_ptr(key)?) })
    }

    #[inline]
    pub fn get_cached_mut(&mut self, key: &K) -> Option<&mut V> {
        Some(unsafe { &mut *(self.get_cached_ptr(key)? as *mut V) })
    }

    #[inline]
    pub fn get_inner(&self) -> &HashMap<K, V, Hasher> {
        &self.inner
    }

    pub fn get_inner_mut(&mut self) -> CachedBorrowMut<K, V> {
        unsafe { (*self.cache.get()).clear() };
        CachedBorrowMut { reference: self }
    }

    pub fn get_or_insert(&mut self, key: &K, mut gen: impl FnMut() -> V) -> &mut V {
        if let Some(x) = self.get_cached_ptr(key) {
            return unsafe { &mut *(x as *mut V) };
        }

        // First invalidate the cache:
        self.cache.get_mut().clear();
        self.inner.insert(*key, gen());
        self.get_cached_mut(key)
            .expect("Should contain a value because we just inserted it")
    }

    pub fn remove(&mut self, key: &K) {
        self.inner.remove(key);
        self.cache.get_mut().clear();
    }
}

impl<K: Copy + Hash + Eq, V> Default for CachedHashMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, K, V> Deref for CachedBorrowMut<'a, K, V> {
    type Target = HashMap<K, V, Hasher>;

    fn deref(&self) -> &HashMap<K, V, Hasher> {
        &self.reference.inner
    }
}

impl<'a, K, V> DerefMut for CachedBorrowMut<'a, K, V> {
    fn deref_mut(&mut self) -> &mut HashMap<K, V, Hasher> {
        &mut self.reference.inner
    }
}

impl<K: Clone, V: Clone> Clone for CachedHashMap<K, V> {
    fn clone(&self) -> Self {
        let inner = self.inner.clone();
        let cache = unsafe { (*self.cache.get()).clone() };
        Self {
            inner,
            cache: UnsafeCell::new(cache),
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        // Generate some operations, and apply them to a raw HashMap and a
        // CachedHashMap.  Then compare.
        #[test]
        fn test_against_hash(operations in prop::collection::vec(
            // (operation, key, value) tuple.
            // Operation is 0=read, 1=write, 2=remove.
            (0..3u8, 0..500u32, 0..1000000u32),
            0..1000,
        )) {
            let mut cached: CachedHashMap<u32, u32> = Default::default();
            let mut good: HashMap<u32, u32, Hasher> = Default::default();

            for (op, k, v) in operations {
                if op == 0 {
                    cached.get_inner_mut().insert(k, v);
                    good.insert(k, v);
                } else if op == 1 {
                    prop_assert_eq!(cached.get_cached(&k), good.get(&k));
                } else if op == 2 {
                    cached.remove(&k);
                    good.remove(&k);
                }

                prop_assert_eq!(&good, &cached.inner);
            }
        }
    }
}
