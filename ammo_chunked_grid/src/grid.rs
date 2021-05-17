//! A compressed grid optimized for a small number of unique objects and for
//! readh speed.  Reads are branchless `O(1)` though with a slight overhead from simply
//! reading the raw arrays.  Writes have a variable complexity and are very
//! slow, but can currently never be better than `O(n * log n)` on the number of
//! unique object values stored in the array.
//!
//! The grid is configured with a default value, which is returned for unwritten
//! cells.  To introduce a concept of `NULL`, use `Option<T>` as with the Rest
//! of Rust and set the default value to `None`.
//!
//! It is *very* important to note that the grid does not store a unique `T` for
//! each cell.  Using interior mutability will modify all cells, not just the
//! one in question.  Updates need to replace values.  This isn't optimized for
//! frequently changing data.
use crate::{u32_grid::*, u32_lut::*};

/// A grid backed by some number of potentially compressed chunks.  See the
/// module level documentation for details.
pub struct Grid<T: Eq + Ord + PartialEq + PartialOrd> {
    data: U32Grid,
    lut: U32Lut<T>,
}

impl<T: Eq + Ord + PartialEq + PartialOrd> Grid<T> {
    fn new(default_value: T) -> Grid<T> {
        let mut lut = U32Lut::new();
        assert_eq!(lut.insert_or_inc_ref(default_value), 0);
        Grid {
            data: U32Grid::new(),
            lut,
        }
    }

    fn read(&self, x: i64, y: i64) -> &T {
        let u = self.data.read(x, y);
        self.lut.translate_out(u)
    }

    fn write(&mut self, x: i64, y: i64, val: T) {
        // Put the new value in, remembering what the old value was.
        let old = self.data.write(x, y, self.lut.insert_or_inc_ref(val));
        // If it's not 0, the default, kill it. We can't unconditionally kill
        // the default: all cells are implicitly set to the default and that
        // will yank it out from under everyone else.
        if old != 0 {
            self.lut.dec_ref(old);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use proptest::prelude::*;

    use super::*;

    /// We want to force proptest to test a few different variations of the data,
    /// but with a common test. This function takes a list of writes of the form
    /// `(x, y, val)` and runs a test against a BTreeMap-backed implementation to
    /// make sure the behaviors always match.
    /// To play nice with proptest, the values are two-tuples instead of a
    /// dedicated struct; this is because tuples are something proptest
    /// strategies already understand.  The more complex default value (as
    /// opposed to a single int) makes sure that we have coverage for later if
    /// this ever decides to do something more complicated than moving thema
    /// round.
    fn test_body(defval: (u32, u32), writes: Vec<(i64, i64, (u32, u32))>) {
        // We will compare the grid against a hashmap implementation.
        let mut good_impl: HashMap<(i64, i64), (u32, u32)> = HashMap::new();
        let mut grid = Grid::<(u32, u32)>::new(defval);

        for (x, y, val) in writes {
            good_impl.insert((x, y), val);
            grid.write(x, y, val);
            assert_eq!(grid.read(x, y), &val);
        }

        // Now check that reading it all back works.
        for ((e_x, e_y), e_val) in good_impl.into_iter() {
            assert_eq!(grid.read(e_x, e_y), &e_val);
        }
    }

    proptest! {
        /// Test what happens when writes are focused into a small area and over a small set of values.
        #[test]
        fn test_constrained(
            defval in (0..2u32, 0..2u32),
            writes in prop::collection::vec((-200i64..200i64, -200i64..200i64, (0..2u32, 0..2u32)), 0..1000)
        ) {
            test_body(defval, writes);
        }

        #[test]
        fn test_large(
            defval in (0..u32::MAX, 0..u32::MAX),
            writes in prop::collection::vec((i64::MIN..i64::MAX, i64::MIN..i64::MAX, (0..u32::MAX, 0..u32::MAX)), 0..1000)
        ) {
            test_body(defval, writes);
        }
    }
}
