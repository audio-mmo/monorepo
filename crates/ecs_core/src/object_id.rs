//! The [ObjectId] type.
//!
//! Object ids are composed of a 64-bit counter which monotonically increments per run of the program, and a 64-bit
//! random value.  In practice, the counter is the number of nanoseconds since the Unix epoch, but this detail is
//! handled by the constructor of the object id and may not hold true for tests. The intent is that the counters
//! (almost) never repeat, and the 64-bit random value is used as a low-quality uuid.
//!
//! In order to play nice with niche value optimizations, the counter must never be zero.
//!
//! In addition to functioning as a tie-breaker to prevent uniqueness, the random component of the id can also be used
//! as a hash.
//!
//! In practice,. though, we want this to play nice with sqlite.  To do that, we make the datastore crate support i128
//! as blobs and provide conversion to/from that representation.
use std::num::NonZeroU64;

// We have to do ord by hand, but equivalence is fine because the underlying values have to be the same.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct ObjectId {
    counter: NonZeroU64,
    random: u64,
}

impl ObjectId {
    pub fn new(counter: u64, random: u64) -> ObjectId {
        assert!(counter != 0);
        ObjectId {
            counter: NonZeroU64::new(counter).expect("Counter must not be zero"),
            random,
        }
    }

    /// Create an object id for testing, with a counter of the specified value and a random portion of zeros.  Should not be used in production.  Public because other crates may wish to use it.
    ///
    /// Panics if the counter is zero, for convenience.
    pub fn new_testing(counter: u64) -> ObjectId {
        let counter = NonZeroU64::new(counter).expect("Counter must not be zero");
        ObjectId { counter, random: 0 }
    }

    pub fn get_counter(&self) -> u64 {
        self.counter.get()
    }

    pub fn get_random(&self) -> u64 {
        self.random
    }
}

impl std::fmt::Debug for ObjectId {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.debug_struct("ObjectId")
            .field("counter", &self.get_counter())
            .field("random", &self.get_random())
            .finish()
    }
}

impl std::cmp::PartialOrd for ObjectId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::cmp::Ord for ObjectId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.get_counter(), self.get_random()).cmp(&(other.get_counter(), other.get_random()))
    }
}

impl From<ObjectId> for i128 {
    fn from(val: ObjectId) -> i128 {
        let urep = (val.counter.get() as u128) << 64 | val.random as u128;
        // Flip the upper bit before transmuting, and this makes the comparison order the same as the non-i128 ids.
        let urep = urep ^ 1 << 127;
        i128::from_ne_bytes(urep.to_ne_bytes())
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Invalid object id {0}")]
pub struct InvalidObjectIdError(i128);

impl TryFrom<i128> for ObjectId {
    type Error = InvalidObjectIdError;

    fn try_from(val: i128) -> std::result::Result<ObjectId, InvalidObjectIdError> {
        let urep = u128::from_ne_bytes(val.to_ne_bytes());
        let urep = urep ^ 1 << 127;
        let counter = (urep >> 64) as u64;
        let random = (urep & u64::MAX as u128) as u64;
        Ok(ObjectId {
            counter: NonZeroU64::new(counter).ok_or(InvalidObjectIdError(val))?,
            random,
        })
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    /// We play tricks with u64->i64->u64 conversions.  Let's make sure that this is as lossless as we think, by trying
    /// to round-trip a table of tricky values.
    #[test]
    fn test_problematic_values() {
        for (c, r) in [
            // Cover the two test cases of one more than i64::MAX.  This can catch endianness issues.
            //
            // Also cover making sure we don't mix up the halves by running it both ways.
            (i64::MAX as u64 + 1, 0),
            (1, i64::MAX as u64 + 1),
            // And then we want to also handle u64::MAX.
            (u64::MAX, 1),
            (1, u64::MAX),
            (u64::MAX, u64::MAX),
        ] {
            let oid = ObjectId::new(c, r);
            assert_eq!((oid.get_counter(), oid.get_random()), (c, r));
        }
    }

    #[test]
    fn test_ordering() {
        use std::cmp::Ordering::*;

        for ((c1, r1), (c2, r2), ordering) in [
            ((1, 0), (2, 0), Less),
            ((1, 1), (1, 1), Equal),
            ((u64::MAX, 0), (1, u64::MAX), Greater),
            ((u64::MAX, u64::MAX), (u64::MAX, u64::MAX), Equal),
        ] {
            let o1 = ObjectId::new(c1, r1);
            let o2 = ObjectId::new(c2, r2);
            assert_eq!(o1.cmp(&o2), ordering, "{:?} {:?}", o1, o2);
            assert_eq!(o1.partial_cmp(&o2), Some(ordering), "{:?} {:?}", o1, o2);
            let i1: i128 = o1.into();
            let i2: i128 = o2.into();
            assert_eq!(i1.cmp(&i2), ordering, "{:?} {:?} {} {}", o1, o2, i1, i2);
        }
    }

    proptest! {
        #[test]
        fn roundtrip(counter in 1..=u64::MAX, random: u64) {
            let o1 = ObjectId::new(counter, random);
            let i: i128 = o1.into();
            let o2 = i.try_into().expect("Should always succeed becuase this is a roundtrip test");
            prop_assert_eq!(o1, o2, "{:?} {:?} {}", o1, o2, i);
        }
    }

    proptest! {
        #[test]
        fn fuzz_ordering_i128(counter1 in 1..=u64::MAX, random1: u64, counter2 in 1..=u64::MAX, random2: u64) {
            let o1 = ObjectId::new(counter1, random1);
            let o2 = ObjectId::new(counter2, random2);
            let ord1 = o1.cmp(&o2);
            let i1: i128 = o1.into();
            let i2 = o2.into();
            let ord2 = i1.cmp(&i2);
            prop_assert_eq!(ord1, ord2);
        }
    }
}
