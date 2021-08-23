//! A `SparseU32Map`  maps u32 keys to values.
//!
//! - Iteration is `O(n)` and as fast as any vec or slice.
//! - Member checks are `O(1)`.
//! - The map iterates in insertion order as long as no deletes are performed,
//!   and iteration of items added after the last delete are visited in insertion
//!   order.
use std::collections::HashMap;

type Hasher = std::hash::BuildHasherDefault<rustc_hash::FxHasher>;

// See: https://research.swtch.com/sparse
//
// The basic idea is that we can check for an element in the set by asking
// `sparse[i]` where it thinks the element may be at, then comparing the element
// in the set.
//
// But we have a twist.  Ammo wants to actually use small sets per area, not one
// big set for the entire loaded map.  As a consequence, we actually use
// `HashMap` to back the indices (not the cached version; we don't expect this
// to use the same key repeatedly).  The uninitialized memory trick works great
// for the case wherein there's a lot of ids, but allocating large pages wastes
// something on the order of 90%+ memory.  We can accelerate this if we need to
// by implementing a custom hashtable, but we care about fast iteration
// performance and we still have that, so punt optimizing down the road since it
// can definitely be done once it becomes necessary.

#[derive(Clone)]
pub struct SparseU32Map<V> {
    dense_keys: Vec<u32>,
    dense_values: Vec<V>,
    sparse_keys: HashMap<u32, u32, Hasher>,
}

impl<V> Default for SparseU32Map<V> {
    fn default() -> Self {
        Self {
            dense_keys: Default::default(),
            dense_values: Default::default(),
            sparse_keys: Default::default(),
        }
    }
}

impl<V> SparseU32Map<V> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn is_empty(&self) -> bool {
        self.dense_keys.is_empty()
    }

    fn dense_index_for_key(&self, key: u32) -> Option<usize> {
        let ind = self.sparse_keys.get(&key).cloned().unwrap_or(0) as usize;
        if self.dense_keys.get(ind) == Some(&key) {
            Some(ind)
        } else {
            None
        }
    }

    pub fn contains(&self, key: u32) -> bool {
        self.dense_index_for_key(key).is_some()
    }

    pub fn get(&self, key: u32) -> Option<&V> {
        let ind = self.dense_index_for_key(key)?;
        Some(unsafe { self.dense_values.get_unchecked(ind) })
    }

    pub fn get_mut(&mut self, key: u32) -> Option<&mut V> {
        let ind = self.dense_index_for_key(key)?;
        self.dense_values.get_mut(ind)
    }

    /// Returns the old value if any.
    pub fn insert(&mut self, key: u32, value: V) -> Option<V> {
        let new_dense_ind = match self.dense_index_for_key(key) {
            Some(ind) => {
                self.dense_keys[ind] = key;
                let mut ov = value;
                std::mem::swap(&mut ov, &mut self.dense_values[ind]);
                return Some(ov);
            }
            None => {
                self.dense_keys.push(key);
                self.dense_values.push(value);
                self.dense_keys.len() - 1
            }
        };

        self.sparse_keys.insert(key, new_dense_ind as u32);
        None
    }

    /// Unsafe because it performs UB if the indices are out of bounds.
    unsafe fn swap(&mut self, first: usize, second: usize) {
        self.dense_keys.swap(first, second);
        self.dense_values.swap(first, second);
        self.sparse_keys
            .insert(*self.dense_keys.get_unchecked(first), first as u32);
        self.sparse_keys
            .insert(*self.dense_keys.get_unchecked(second), second as u32);
    }

    pub fn remove(&mut self, key: u32) -> Option<V> {
        let dense_ind = self.dense_index_for_key(key)?;
        let end_ind = self.dense_keys.len() - 1;
        unsafe {
            self.swap(dense_ind, end_ind);
        };
        match self.dense_keys.pop() {
            Some(_) => (),
            None => unsafe { std::hint::unreachable_unchecked() },
        }
        match self.dense_values.pop() {
            Some(x) => Some(x),
            None => unsafe { std::hint::unreachable_unchecked() },
        }
    }

    pub fn keys<'a>(&'a self) -> impl Iterator<Item = &u32> + 'a {
        self.dense_keys.iter()
    }

    pub fn values<'a>(&'a self) -> impl Iterator<Item = &V> + 'a {
        self.dense_values.iter()
    }

    pub fn values_mut<'a>(&'a mut self) -> impl Iterator<Item = &mut V> + 'a {
        self.dense_values.iter_mut()
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = (&u32, &V)> + 'a {
        (&self.dense_keys[..])
            .iter()
            .zip((&self.dense_values[..]).iter())
    }

    /// Iterate over `(&K, &mut V)` tuples.
    ///
    /// being able to mutate keys isn't currently supported because this can be used to break set invariants.
    pub fn iter_kv_mut(&mut self) -> impl Iterator<Item = (&u32, &mut V)> {
        let kslice = &self.dense_keys[..];
        let vslice = &mut self.dense_values[..];
        kslice.iter().zip(vslice.iter_mut())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::HashMap;

    use proptest::prelude::*;

    #[derive(Debug)]
    enum Operation {
        Insert(u32, u32),
        Delete(u32),
    }

    proptest::prop_compose! {
        fn operation(max_key: u32, max_value: u32)(
            op in 0..1usize,
            key in 0..max_key,
            value in 0..max_value) -> Operation {
            if op == 0 {
                Operation::Insert(key, value)
            } else if op == 1 {
                Operation::Delete(key)
            } else {
                unreachable!()
            }
        }

    }

    fn test_impl(max_key: u32, max_val: u32, times: usize) {
        proptest!(move |(ops in prop::collection::vec(operation(max_key, max_val), 0..times))| {
            let mut set = SparseU32Map::new();
            let mut good = HashMap::<u32, u32>::new();

            for o in ops.into_iter() {
                match o {
                    Operation::Insert(key, val) => {
                        prop_assert_eq!(set.insert(key, val), good.insert(key, val));
                    }
                    Operation::Delete(key) => {
                        prop_assert_eq!(set.remove(key), good.remove(&key));
                    }
                }

                // Now check a bunch of invariants.
                for (k, _) in good.iter() {
                    prop_assert_eq!(set.get(*k), good.get(k));
                }
                for (k, _) in set.iter() {
                    prop_assert_eq!(set.get(*k), good.get(k));
                }

                let mut set_vec = set.iter().collect::<Vec<_>>();
                let mut good_vec = good.iter().collect::<Vec<_>>();
                set_vec.sort_unstable();
                good_vec.sort_unstable();
                prop_assert_eq!(set_vec, good_vec);
            }
        });
    }

    #[test]
    fn test_small() {
        test_impl(5, 5, 1000);
    }

    #[test]
    fn test_sparse() {
        test_impl(1000000, 5, 1000);
    }
}
