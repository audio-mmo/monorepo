//! A `SparseMap`  provides the ability to map anything which can be converted
//! to a `usize` to a single value.  Specifically:
//!
//! - Iteration is `O(n)` and as fast as any vec or slice.
//! - Member checks are `O(1)`.
//! - The set iterates in insertion order as long as no deletes are performed,
//!   and iteration of items added after the last delete are visited in sorted
//!   order.
//! - The set can be sorted by the index of the key type.

const PAGE_SIZE: usize = 8192;
const PAGE_MASK: usize = !(PAGE_SIZE - 1);

/// A trait which encapsulates the concept of transforming a struct to aa `usize` index.
///
/// This isn't using `From` because it is an important property that the `usize` index never change for the lifetime of the item (re.g. if the item is internally mutable, it must not change this index).
///
/// Specifically:
/// - Any key must never change index after creation;
/// - `a.to_index() == b.to_index()` if a and b are the same.
pub trait SparseMapKey: Eq {
    fn to_index(&self) -> usize;
}

impl SparseMapKey for usize {
    fn to_index(&self) -> usize {
        *self
    }
}

fn key_to_page_index<K: SparseMapKey>(key: &K) -> (usize, usize) {
    let ind = key.to_index();
    (ind & PAGE_MASK, ind & !PAGE_MASK)
}

type SparsePages = ammo_cached_hash_map::CachedHashMap<usize, [usize; PAGE_SIZE]>;

// See: https://research.swtch.com/sparse
//
// The basic idea is that we can check for an element in the set by asking
// `sparse[i]` where it thinks the element may be at, then comparing the element
// in the set.  Note that this isn't just ints, so we are a little bit more
// complicated than that.

#[derive(Clone)]
pub struct SparseMap<K: SparseMapKey, V> {
    dense_keys: Vec<K>,
    dense_values: Vec<V>,
    sparse: SparsePages,
}

impl<K: SparseMapKey, V> Default for SparseMap<K, V> {
    fn default() -> Self {
        Self {
            dense_keys: Default::default(),
            dense_values: Default::default(),
            sparse: Default::default(),
        }
    }
}

impl<K: SparseMapKey, V> SparseMap<K, V> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn is_empty(&self) -> bool {
        self.dense_keys.is_empty()
    }

    fn dense_index_for_key(&self, key: &K) -> Option<usize> {
        if self.is_empty() {
            return None;
        }

        let (page, index) = key_to_page_index(key);
        if let Some(dense_page) = self.sparse.get_cached(&page) {
            let dense_ind = dense_page[index];
            if dense_ind < self.dense_keys.len() && key == &self.dense_keys[dense_ind] {
                Some(dense_ind)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn contains(&self, key: &K) -> bool {
        self.dense_index_for_key(key).is_some()
    }

    fn ensure_page(&mut self, page: usize) -> &mut [usize; PAGE_SIZE] {
        self.sparse.get_or_insert(&page, || [0; PAGE_SIZE])
    }

    /// Returns the old value if any.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let (page, page_ind) = key_to_page_index(&key);

        let (new_dense_ind, old_val) = match self.dense_index_for_key(&key) {
            Some(ind) => {
                self.dense_keys[ind] = key;
                let mut ov = value;
                std::mem::swap(&mut ov, &mut self.dense_values[ind]);
                (ind, Some(ov))
            }
            None => {
                self.dense_keys.push(key);
                self.dense_values.push(value);
                (self.dense_keys.len() - 1, None)
            }
        };

        let page = self.ensure_page(page);
        page[page_ind] = new_dense_ind;
        old_val
    }

    fn swap_indices(&mut self, first: usize, second: usize) {
        self.dense_keys.swap(first, second);
        self.dense_values.swap(first, second);
        let (p1, i1) = key_to_page_index(&self.dense_keys[first]);
        let (p2, i2) = key_to_page_index(&self.dense_keys[second]);
        if p1 == p2 {
            let page = self.ensure_page(p1);
            page[i1] = first;
            page[i2] = second;
        } else {
            self.ensure_page(p1)[i1] = first;
            self.ensure_page(p2)[i2] = second;
        }
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        let dense_ind = self.dense_index_for_key(key)?;
        let end_ind = self.dense_keys.len() - 1;
        self.swap_indices(dense_ind, end_ind);
        self.dense_keys
            .pop()
            .expect("Should delete becasue the set isn't empty");
        self.dense_values.pop()
    }

    pub fn keys<'a>(&'a self) -> impl Iterator<Item = &K> + 'a {
        self.dense_keys.iter()
    }

    pub fn values<'a>(&'a self) -> impl Iterator<Item = &V> + 'a {
        self.dense_values.iter()
    }

    pub fn values_mut<'a>(&'a mut self) -> impl Iterator<Item = &mut V> + 'a {
        self.dense_values.iter_mut()
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = (&K, &V)> + 'a {
        (0..self.dense_keys.len()).map(move |i| (&self.dense_keys[i], &self.dense_values[i]))
    }

    /// Iterate over `(&K, &mut V)` tuples.
    ///
    /// being able to mutate keys isn't currently supported because this can be used to break set invariants.
    pub fn iter_kv_mut<'a>(&'a mut self) -> impl Iterator<Item = (&'a K, &'a mut V)> + 'a {
        // The borrow checker is not smart enough here to understand that the lifetimes borrowed by the closure are disjoint, so we must sadly help it with unsafe.
        let kptr = &self.dense_keys as *const Vec<K>;
        let vptr = &mut self.dense_values as *mut Vec<V>;
        (0..self.dense_values.len()).map(move |i| -> (&'a K, &'a mut V) {
            unsafe {
                (
                    (*kptr).get_unchecked(i) as &'a K,
                    (*vptr).get_unchecked_mut(i) as &'a mut V,
                )
            }
        })
    }
}
