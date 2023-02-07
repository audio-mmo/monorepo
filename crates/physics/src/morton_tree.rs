use std::num::NonZeroUsize;

use slab::Slab;

use crate::morton::*;

/// A tree mapping [MortonPrefix]s to nodes.
///
/// This tree supports the normal operations, but also the ability to walk to the ancestors or children of any prefix.  Each unique prefix gets a unique value, so things wishing to store more than one value use things like smallvec.

struct MortonTree<T> {
    root: Option<SlabRef>,
    node_slab: Slab<Node>,
    value_slab: Slab<T>,
}

/// Wrapper type to add a niche to keys from [Slab]s.
#[derive(Copy, Clone)]
struct SlabRef(NonZeroUsize);

struct Node {
    value: Option<SlabRef>,
    children: [Option<SlabRef>; 4],
}

impl SlabRef {
    fn new(key: usize) -> SlabRef {
        SlabRef(NonZeroUsize::new(key + 1).unwrap())
    }

    fn get_key(&self) -> usize {
        self.0.get() - 1
    }
}

impl<T> MortonTree<T> {
    pub fn new() -> Self {
        MortonTree {
            root: None,
            node_slab: Slab::new(),
            value_slab: Slab::new(),
        }
    }

    fn slab_ref_for_node(&self, prefix: &MortonPrefix) -> Option<SlabRef> {
        let mut cur = self.root?;

        for i in prefix.unpack() {
            let n = self
                .node_slab
                .get(cur.get_key())
                .expect("Nodes should exist in the slab");
            cur = n.children[i as usize]?;
        }

        Some(cur)
    }

    fn slab_ref_for_value(&self, prefix: &MortonPrefix) -> Option<SlabRef> {
        let node = self.slab_ref_for_node(prefix)?;
        self.node_slab[node.get_key()].value
    }

    pub fn get(&self, prefix: &MortonPrefix) -> Option<&T> {
        Some(&self.value_slab[self.slab_ref_for_value(prefix)?.get_key()])
    }

    pub fn get_mut(&mut self, prefix: &MortonPrefix) -> Option<&mut T> {
        let k = self.slab_ref_for_value(prefix)?;
        Some(&mut self.value_slab[k.get_key()])
    }

    /// Ensure that a chain of nodes leading to prefix is allocated in the tree, returning a reference to the final node in the chain.
    fn ensure_node(&mut self, prefix: &MortonPrefix) -> SlabRef {
        // Point at the root, allocating it if it hasn't been yet.
        let mut cur = match self.root {
            Some(x) => x.get_key(),
            None => {
                let allocated = self.node_slab.insert(Node {
                    value: None,
                    children: Default::default(),
                });
                self.root = Some(SlabRef::new(allocated));
                allocated
            }
        };

        // Now, we descend in that manner until we hit the end of the prefix...
        for i in prefix.unpack() {
            let child = self.node_slab[cur].children[i as usize];
            cur = match child {
                Some(c) => c.get_key(),
                None => {
                    let allocated = self.node_slab.insert(Node {
                        value: None,
                        children: Default::default(),
                    });
                    self.node_slab[cur].children[i as usize] = Some(SlabRef::new(allocated));
                    allocated
                }
            }
        }

        SlabRef::new(cur)
    }

    /// Insert a value into the tree, returning the previous value if any.
    pub fn insert(&mut self, prefix: &MortonPrefix, mut value: T) -> Option<T> {
        let node = self.ensure_node(prefix);
        match self.node_slab[node.get_key()].value {
            Some(x) => {
                std::mem::swap(&mut value, &mut self.value_slab[x.get_key()]);
                Some(value)
            }
            None => {
                let val = self.value_slab.insert(value);
                self.node_slab[node.get_key()].value = Some(SlabRef::new(val));
                None
            }
        }
    }

    /// Delete a given value from the tree.
    ///
    /// This does not clear the nodes. After bulk operations, call [Self::prune_empty].
    pub fn remove(&mut self, prefix: &MortonPrefix) -> Option<T> {
        let node = self.slab_ref_for_node(prefix)?;
        let vk = self.node_slab[node.get_key()].value?;
        self.node_slab[node.get_key()].value = None;
        Some(self.value_slab.remove(vk.get_key()))
    }

    /// Compact the subtree starting at root.
    ///
    /// returns whether the tree was dropped.
    fn prune_subtree(&mut self, root: SlabRef) -> bool {
        let children = self.node_slab[root.get_key()].children;
        let mut pruning = true;
        for c in children.into_iter().flatten() {
            pruning &= self.prune_subtree(c);
        }

        pruning &= self.node_slab[root.get_key()].value.is_none();

        if pruning {
            self.node_slab.remove(root.get_key());
        }

        pruning
    }

    pub fn clear(&mut self) {
        self.root = None;
        self.node_slab.clear();
        self.value_slab.clear();
    }

    pub fn num_nodes(&self) -> usize {
        self.node_slab.len()
    }

    pub fn num_values(&self) -> usize {
        self.value_slab.len()
    }
}

impl<T> Default for MortonTree<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    use proptest::prelude::*;
    use proptest::{prop_assert, prop_assert_eq};

    use std::collections::HashSet;

    // Fuzz the tree.
    //
    // Unfortunately it is hard to generate interesting cases.  For now we'll just proptest the hell out of it.
    proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig {
            cases: 10000,
            ..Default::default()
        })]

        #[test]
        fn test_fuzz(
            prefixes in proptest::collection::vec(
                proptest::arbitrary::any::<MortonPrefix>(),
                1..10000usize
            )
        ) {
            let mut tree = MortonTree::new();

            for p in prefixes.iter().cloned() {
                tree.insert(&p, p);
            }

            for p in prefixes.iter() {
                prop_assert_eq!(tree.get(p), Some(p));
            }

            let unique_prefixes = prefixes.iter().cloned().collect::<HashSet<_>>();
            for p in unique_prefixes.iter() {
                tree.remove(p).expect("Value should be in the tree");
                prop_assert!(tree.get(p).is_none());
            }

            for p in unique_prefixes.iter() {
                prop_assert!(tree.get(p).is_none());
            }

            prop_assert_eq!(tree.num_values(), 0);

        }
    }
}
