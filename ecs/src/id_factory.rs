//! The `IdFactory` produces object ids using an approximation of the number of nanoseconds since the Unix epoch.
//!
//! This is done by reading the SystemTime at object construction to compute a base counter value, then incrementing the
//! counter per id generation.  Things are kept in sync using a rate limiter configured so that no more than 1e9
//! generations can happen per second.
//!
//! Obviously, this means the "clock" can fall behind if fewer than the maximum number of object ids are generated, but
//! this is okay: it's only important that we not jump ahead, so that when the program restarts and the counter re-syncs
//! with time there's minimal overlap between the last run in this one.  In practice, we expect no overlap at all when
//! run on a server with proper time configuration.
use std::num::NonZeroU64;
use std::sync::Mutex;

use governor::{Quota, RateLimiter};
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;

use crate::object_id::ObjectId;

/// The default rate limiter quota for the id factory, which allows 100m through per second.  This is sufficient for
/// production since reading randomness should take much longer than this, and has the nice property that time always
/// falls behind.
const DEFAULT_QUOTA: Quota =
    unsafe { Quota::per_second(std::num::NonZeroU32::new_unchecked(100000000)) };

/// An `IdFactory` builds object ids from a base counter and some randomness.
#[derive(Debug)]
pub struct IdFactoryState {
    limiter: RateLimiter<
        governor::state::direct::NotKeyed,
        governor::state::InMemoryState,
        governor::clock::MonotonicClock,
    >,
    counter: NonZeroU64,
    rng: ChaCha8Rng,
}

#[derive(Debug)]
pub struct IdFactory {
    inner: Mutex<IdFactoryState>,
}

impl IdFactory {
    /// generate an id factory from the system timestamp and some randomness.
    pub fn new() -> IdFactory {
        use std::convert::TryFrom;

        let now = std::time::SystemTime::now();
        let since_epoch = now
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .expect("Shouldn't be at the UNIX epoch");
        let seed = rand::thread_rng().gen();
        let ns = since_epoch.as_nanos();
        Self::new_with_params(
            unsafe {
                NonZeroU64::new_unchecked(u64::try_from(ns).expect(
                    "Should be able to convert to u64 because we aren't too far ahead of realtime",
                ))
            },
            seed,
            DEFAULT_QUOTA,
        )
    }

    /// generate an IdFactory from a seed and base counter.
    pub fn new_with_params(counter: NonZeroU64, seed: u64, quota: Quota) -> IdFactory {
        let inner = IdFactoryState {
            counter,
            rng: ChaCha8Rng::seed_from_u64(seed),
            limiter: RateLimiter::direct_with_clock(quota, &Default::default()),
        };
        IdFactory {
            inner: Mutex::new(inner),
        }
    }

    /// Try to generate an object id. Fails with the time at which an object id should next be available.
    pub fn try_generate_id(&self) -> Result<ObjectId, std::time::Instant> {
        let mut guard = self.inner.lock().unwrap();
        if let Err(e) = guard.limiter.check() {
            return Err(e.earliest_possible());
        }

        let r = guard.rng.gen();
        let c = guard.counter.get();
        // checked_add and everything else we might use on NonZeroU64 is nightly-only.
        let next = guard
            .counter
            .get()
            .checked_add(1)
            .expect("Shouldn't wrap around until the year 2554");
        guard.counter = unsafe { NonZeroU64::new_unchecked(next) };
        Ok(ObjectId::new(c, r))
    }

    /// Generate an ObjectId, spin-waiting and yielding to other threads if the governor says we can't.
    ///
    /// Panics if the counter has reached the maximum value, which can't happen until the year 2554.  We've done
    /// exceptionally well if this is a problem.
    pub fn generate_id(&self) -> ObjectId {
        loop {
            if let Ok(i) = self.try_generate_id() {
                return i;
            }
            std::thread::yield_now();
        }
    }
}

impl Default for IdFactory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Check that the counter of object ids always increments.
    #[test]
    fn test_monotonic() {
        let fact = IdFactory::new();
        let id1 = fact.generate_id();
        let id2 = fact.generate_id();
        let id3 = fact.generate_id();
        let d1 = id2.get_counter() - id1.get_counter();
        let d2 = id3.get_counter() - id2.get_counter();
        assert_eq!(d1, 1);
        assert_eq!(d2, 1);
    }

    #[test]
    fn test_no_duplicate_randomness() {
        let fact = IdFactory::new();
        let rands = (0..100).map(|_| fact.generate_id().get_random());
        let hs = rands.collect::<std::collections::HashSet<_>>();
        assert_eq!(hs.len(), 100);
    }

    #[test]
    fn test_rate_limiting() {
        let quota = Quota::per_minute(std::num::NonZeroU32::new(5).unwrap());
        let fact = IdFactory::new_with_params(NonZeroU64::new(1).unwrap(), 10, quota);
        for _ in 0..5 {
            let _ = fact.try_generate_id();
        }
        assert!(matches!(fact.try_generate_id(), Err(_)));
    }
}
