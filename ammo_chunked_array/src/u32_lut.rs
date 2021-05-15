//! A `T->u32` LUT optimize for a small number of `T` and for the inverse
//! `u32->T` lookup, where the table itself assigns the ID.  Used to amortize
//! array element storage by only ever storing a `u32` in the array itself.
struct LookupTableEntry<T: Eq + Ord + PartialEq + PartialOrd> {
    external_value: T,
    /// When this goes to zero, deallocate this entry for reuse. Used to prevent
    /// the maximum value in the table growing forever.
    refcount: usize,
}

/// A bidirectional LUT for a `T` -> `u32` mapping.  This works by maintaining a
/// sorted vec mapping `T` to `u32`, and a sorted vec mapping a `u32` to the
/// slot in the `T -> u32` table that can be used to invert the lookup.
///
/// Entries are reused once they are no longer useful.
///
/// To use, call either `translate_out` (the immutable
/// interface) or `insert_or_inc_ref` and `dec_ref` for insertion/refcount
/// management.  Mixing these up is bad.
///
/// because the chunked array doesn't need an immutable `translate_in`, we only provide `translate_out`.
///
/// Note that this table panics if used incorrectly: it is for internal use only.
struct U32Lut<T: Eq + Ord + PartialEq + PartialOrd> {
    // The entries. The value assigned to any particular `T` is the position in this array, and elements never move.
    entries: Vec<LookupTableEntry<T>>,
    /// An index of sorted integers where the elements of the list are indexes
    /// into the entries map, such that this list is sorted by `T`. Used for
    /// inverse translations.
    inverse_index: Vec<usize>,
    /// Indices that we can reuse.
    freelist: Vec<usize>,
}

impl<T: Eq + Ord + PartialEq + PartialOrd> Default for U32Lut<T> {
    fn default() -> Self {
        U32Lut {
            entries: Default::default(),
            inverse_index: Default::default(),
            freelist: Default::default(),
        }
    }
}

impl<T: Eq + Ord + PartialEq + PartialOrd> U32Lut<T> {
    fn new() -> Self {
        Default::default()
    }

    fn translate_out(&self, value: u32) -> &T {
        &self.entries[value as usize].external_value
    }

    /// insert a `T` into the map if ossible.  Return the assigned `u32` value.
    fn insert_or_inc_ref(&mut self, val: T) -> u32 {
        // Let's try to just get the current one if possible.
        if let Ok(i) = self
            .inverse_index
            .binary_search_by(|a| self.entries[*a].external_value.cmp(&val))
        {
            let ind = self.inverse_index[i];
            self.entries[ind].refcount += 1;
            return ind as u32;
        }

        // Otherwise, can we get an index from the freelist?
        let retval;
        if let Some(i) = self.freelist.pop() {
            self.entries[i].refcount = 1;
            self.entries[i].external_value = val;
            retval = i;
        } else {
            retval = self.entries.len();
            self.entries.push(LookupTableEntry {
                external_value: val,
                refcount: 1,
            });
        }

        // There is room for optimization when rebuilding the index, but that's
        // complicated and inserts of new values are supposed to be rare, so
        // let's not.

        // We have to satisfy the borrow checker unfortunately. Rust 2021 will fix
        // this case, but for now we need to swap the freelist out. Recall that
        // `Vec` doesn't allocate at all until used.
        let mut inv_ind = std::mem::replace(&mut self.inverse_index, vec![]);
        inv_ind.push(retval);
        inv_ind.sort_unstable_by(|a, b| {
            self.entries[*a]
                .external_value
                .cmp(&self.entries[*b].external_value)
        });
        self.inverse_index = inv_ind;

        retval as u32
    }

    fn dec_ref(&mut self, val: u32) {
        // The value is the index.
        let ent = &mut self.entries[val as usize];
        ent.refcount -= 1;
        if ent.refcount == 0 {
            self.freelist.push(val as usize);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Eq, Ord, PartialEq, PartialOrd)]
    struct Ent(u32);

    #[test]
    fn test_u32_lut() {
        let mut lut = U32Lut::<Ent>::new();

        // Insert 10 entries, which should all come back as their index.
        for i in 0..10 {
            let e = Ent(i);
            let got = lut.insert_or_inc_ref(e);
            assert_eq!(got, i);
        }

        // translating values back out should work.
        for i in 0..10 {
            let got = lut.translate_out(i);
            assert_eq!(got.0, i);
        }

        // inserting a value we know to be present gives us the same index.
        let got = lut.insert_or_inc_ref(Ent(3));
        assert_eq!(got, 3, "{:?}", lut.inverse_index);
    }
}
