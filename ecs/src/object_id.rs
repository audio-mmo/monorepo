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
use std::num::NonZeroU64;

#[derive(Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Hash)]
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
            .field("counter", &self.counter.get())
            .field("random", &self.random)
            .finish()
    }
}
