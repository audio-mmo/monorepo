use std::borrow::Borrow;
use std::collections::HashMap;
use std::hash::Hash;

use ahash::RandomState;

type InnerMap<K, V> = HashMap<K, V, RandomState>;

/// A `HashMap` wrapper which uses ahash as the underlying hash function and hides all mutable methods.
///
/// Used to guarantee correctness when working with component stores and systems: after a building phase, it should
/// become impossible to add new ones while the simulation is running.
///
/// This only guarantees that the map itself cannot be mutated; mutating the values is permitted.
pub struct FrozenMap<K, V>(InnerMap<K, V>);

/// Builder for a frozen map.  After calling [FrozenMapBuilder::freeze], the map cannot be modified.
pub struct FrozenMapBuilder<K, V>(InnerMap<K, V>);

impl<K: Hash + Eq, V> FrozenMap<K, V> {
    pub fn get<Q: ?Sized>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.0.get(key)
    }

    pub fn get_mut<Q: ?Sized>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.0.get_mut(key)
    }

    pub fn contains_key<Q: ?Sized>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.0.contains_key(key)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.0.keys()
    }

    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.0.values()
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.0.values_mut()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&K, &mut V)> {
        self.0.iter_mut()
    }
}

impl<K: Eq + Hash, V> FrozenMapBuilder<K, V> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add(&mut self, key: K, value: V) -> &mut Self {
        self.0.insert(key, value);
        self
    }

    pub fn freeze(self) -> FrozenMap<K, V> {
        FrozenMap(self.0)
    }
}

impl<K, V> Default for FrozenMapBuilder<K, V> {
    fn default() -> Self {
        FrozenMapBuilder(Default::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        let mut builder = FrozenMapBuilder::<u32, u32>::new();
        builder.add(1, 2).add(3, 4).add(5, 6);
        let map = builder.freeze();
        assert_eq!(map.get(&1).unwrap(), &2);
        assert_eq!(map.get(&3).unwrap(), &4);
        assert_eq!(map.get(&5).unwrap(), &6);
    }
}
