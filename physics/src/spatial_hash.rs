//! A simple HashMap-backed spatial hash.
use std::collections::HashMap;

use smallvec::SmallVec;

use crate::*;

/// A `SpatialHash` can tell us what items are within the bounds of a particular
/// bounding box.
///
/// This hash doesn't support updates or removals; for the time being, we
/// rebuild them on every tick.
///
/// Every spatial hash divides the world into chunks of a given width and
/// height, then adds items to all the chunks they might be within.  In order to
/// be efficient, chunks should generally be larger than the objects the hash
/// contains, while not being so large that they contain large chunks of the
/// world.  Spatial hashes work best when they are used for situations where all
/// objects are roughly the same size.
///
/// Currently, this is the broadphase algorithm for this crate.  We may wish to
/// change that in future: a slab-backed tree may be better.  But this is
/// absurdly simple to implement.
#[derive(Debug)]
pub struct SpatialHash<T: Clone> {
    entries: HashMap<(i64, i64), SmallVec<[T; 64]>>,
    cell_width: u32,
    cell_height: u32,
}

/// Represents a range in the hash that the box covers: `lx..=hx` and `ly..=hy`.
/// The ranges are inclusive on both ends.
struct RoundedAabb {
    lx: i64,
    ly: i64,
    hx: i64,
    hy: i64,
}

impl<T: Clone> SpatialHash<T> {
    pub fn new(cell_width: u32, cell_height: u32) -> SpatialHash<T> {
        SpatialHash {
            cell_width,
            cell_height,
            entries: Default::default(),
        }
    }

    fn round_aabb(&self, aabb: &Aabb) -> RoundedAabb {
        RoundedAabb {
            lx: (aabb.get_p1().x / self.cell_width as f64).floor() as i64,
            ly: (aabb.get_p1().y / self.cell_height as f64).floor() as i64,
            hx: (aabb.get_p2().x / self.cell_width as f64).floor() as i64,
            hy: (aabb.get_p2().y / self.cell_height as f64).floor() as i64,
        }
    }

    pub fn insert(&mut self, aabb: &Aabb, val: T) {
        let RoundedAabb { lx, ly, hx, hy } = self.round_aabb(aabb);
        for x in lx..=hx {
            for y in ly..=hy {
                self.entries.entry((x, y)).or_default().push(val.clone());
            }
        }
    }

    /// get an iterator over all possible objects that an aabb might cover. Note
    /// that this iterator can and usually does return duplicates.
    pub fn get_items_for_aabb(&self, aabb: &Aabb) -> impl Iterator<Item = &T> {
        let RoundedAabb { lx, ly, hx, hy } = self.round_aabb(aabb);
        // create an iterator over the outer range.
        (lx..=hx)
            .flat_map(move |x| {
                // Now zip the x with an iterator over the inner range.
                (ly..=hy).map(move |y| (x, y))
            })
            .filter_map(move |coord| self.entries.get(&coord))
            .flat_map(|e| e.iter())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // We don't have many good test options which aren't just recomputing the hash with the same algorithm; let's just manipulate the hash to a known state and see if it's right.
    //
    // Later, we will be able to do collision detection testing with the help of proptest, which will fuzz the code paths heavily.
    #[test]
    fn basic_hashing() {
        let aabb1 = Aabb::from_points(V2::new(-5.0, -3.0), V2::new(5.0, 5.0)).unwrap();
        let aabb2 = Aabb::from_points(V2::new(-10.0, -1.0), V2::new(10.0, 1.0)).unwrap();

        // It is important that the width and height be different here, as this can catch division issues.
        let mut hash = SpatialHash::<u32>::new(2, 3);
        hash.insert(&aabb1, 1);
        hash.insert(&aabb2, 2);

        let mut expected = HashMap::<(i64, i64), SmallVec<[u32; 64]>>::new();

        {
            let mut ins = |(x, y), val| {
                expected.entry((x, y)).or_default().push(val);
            };

            // Data for the first box.
            for i in -3..=2 {
                for j in -1..=1 {
                    ins((i, j), 1);
                }
            }

            // And the other one:
            for i in -5..=5 {
                for j in -1..=0 {
                    ins((i, j), 2);
                }
            }
        }

        // Iterating and sorting should produce the right count of each box:
        let a1_count = hash.get_items_for_aabb(&aabb1).filter(|x| **x == 1).count();
        let a2_count = hash.get_items_for_aabb(&aabb2).filter(|x| **x == 2).count();
        assert_eq!(a1_count, 18);
        assert_eq!(a2_count, 22);

        let SpatialHash { entries, .. } = hash;
        let mut flat_entries = entries.into_iter().collect::<Vec<_>>();
        flat_entries.sort_unstable_by_key(|x| x.0);
        let mut flat_expected = expected.into_iter().collect::<Vec<_>>();
        flat_expected.sort_unstable_by_key(|x| x.0);
        pretty_assertions::assert_eq!(flat_entries, flat_expected);
    }
}
