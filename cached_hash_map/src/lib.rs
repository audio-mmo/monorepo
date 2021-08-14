//! A [CachedHashMap] is a [HashMap] replacement which defers to an
//! internal cache before reading the key.
//!
//! So long as the hashmap is immutably borrowed, the cache remains valid and
//! uses a bit of unsafe iun order to return direct pointers to the entries.
//! When a mutable borrow is taken, the cache is invalidated until the next time
//! there are only immutable references.  
//!
//! Finally, this hashmap uses ahash because it is already vulnerable to DOS:
//! reading different items is much slower than reading the same item.
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::ops::{Deref, DerefMut};

struct HashCache<K, V> {
    key: K,
    value: *const V,
}

pub struct CachedHashMap<K, V> {
    inner: HashMap<K, V, ahash::RandomState>,
    cache: UnsafeCell<Option<HashCache<K, V>>>,
    has_mut_borrow: bool,
}

pub struct CachedBorrowMut<'a, K, V> {
    reference: &'a mut CachedHashMap<K, V>,
}

impl<K: Eq + Copy + Hash, V> CachedHashMap<K, V> {
    pub fn new() -> CachedHashMap<K, V> {
        CachedHashMap {
            inner: Default::default(),
            cache: UnsafeCell::new(None),
            has_mut_borrow: false,
        }
    }

    fn get_cached_ptr(&self, key: &K) -> Option<*const V> {
        unsafe {
            match &*self.cache.get() {
                Some(c) if !self.has_mut_borrow && &c.key == key => Some(c.value),
                _ => {
                    let nk = self.inner.get(key)?;
                    if !self.has_mut_borrow {
                        *self.cache.get() = Some(HashCache {
                            key: *key,
                            value: nk as *const V,
                        });
                    }
                    Some(nk as *const V)
                }
            }
        }
    }

    /// Override of [HashMap::get] which tries the cache first.
    pub fn get_cached(&self, key: &K) -> Option<&V> {
        Some(unsafe { &*(self.get_cached_ptr(key)?) })
    }

    pub fn get_inner(&self) -> &HashMap<K, V, ahash::RandomState> {
        &self.inner
    }

    pub fn get_inner_mut(&mut self) -> CachedBorrowMut<K, V> {
        unsafe { *self.cache.get() = None };
        self.has_mut_borrow = true;
        CachedBorrowMut { reference: self }
    }
}

impl<K: Copy + Hash + Eq, V> Default for CachedHashMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, K, V> Drop for CachedBorrowMut<'a, K, V> {
    fn drop(&mut self) {
        self.reference.has_mut_borrow = false;
    }
}

impl<'a, K, V> Deref for CachedBorrowMut<'a, K, V> {
    type Target = HashMap<K, V, ahash::RandomState>;

    fn deref(&self) -> &HashMap<K, V, ahash::RandomState> {
        &self.reference.inner
    }
}

impl<'a, K, V> DerefMut for CachedBorrowMut<'a, K, V> {
    fn deref_mut(&mut self) -> &mut HashMap<K, V, ahash::RandomState> {
        &mut self.reference.inner
    }
}
