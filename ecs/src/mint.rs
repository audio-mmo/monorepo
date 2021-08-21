//! The [mint] produces object ids as needed, and accepts object ids back from anything else that may wish to return them.
//!
//! This is a threadsafe object, which is typically global per process, unless the user is e.g. building ares with ephemeral ids.
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{Mutex, RwLock};

use crate::object_id::ObjectId;

#[derive(Debug)]
pub struct Mint {
    /// The next id we have yet to use.
    next_free_id: AtomicU32,
    /// Any object ids wich have been returned to us.  Only touched when an id
    /// is returned, or when it's necessary to refill the available ids.  Prevents contention on the `RwLock`.
    ///
    /// The ids in this vec have already had their ids incremented.
    returned_ids: Mutex<Vec<ObjectId>>,
    /// Available ids for use by anyone asking for one.
    ///
    // The ids in this vec have already had their versions incremented.
    available_ids: RwLock<Vec<ObjectId>>,
    /// The current number of available ids.
    num_available_ids: AtomicUsize,
    /// Whether produced ids should be ephemeral.
    ephemeral: bool,
}

impl Mint {
    /// Create a [Mint].
    ///
    /// If `ephemeral` is true, returned ids will have their ephemeral bit set.
    pub fn new(ephemeral: bool) -> Mint {
        Self {
            available_ids: RwLock::new(Vec::new()),
            ephemeral,
            num_available_ids: AtomicUsize::new(0),
            next_free_id: AtomicU32::new(0),
            returned_ids: Mutex::new(Vec::new()),
        }
    }

    /// Increment and return the next objecrt id which has enver been used before.
    fn next_never_used_id(&self) -> ObjectId {
        let nid = self.next_free_id.fetch_add(1, Ordering::Relaxed);
        ObjectId::new(nid, 0, self.ephemeral)
    }

    /// Consume any incoming object ids, storing them in the `RwLock` side of the queues.
    fn consume_incoming(&self) {
        let returned = {
            let mut guard = self.returned_ids.lock().unwrap();
            let mut out = vec![];
            std::mem::swap(&mut out, &mut *guard);
            out
        };

        {
            let mut guard = self.available_ids.write().unwrap();
            let tmp = returned.len();
            *guard = returned;
            self.num_available_ids.store(tmp, Ordering::Relaxed);
        }
    }

    /// Generate a fresh object id which is not equal to any other ever produce by this mint.
    ///
    /// Panicks if this is not possible, which can only happen if `u32::Max / 2` objects have been created without returning any ids to the queue.
    pub fn generate_id(&self) -> ObjectId {
        self.generate_id_impl(false)
    }

    fn generate_id_impl(&self, has_consumed_queues: bool) -> ObjectId {
        {
            // First, try to get an id from the queue.
            let guard = self.available_ids.read().unwrap();
            let maybe_old_num =
                self.num_available_ids
                    .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |val| {
                        if val == 0 {
                            None
                        } else {
                            Some(val - 1)
                        }
                    });
            if let Ok(old_num) = maybe_old_num {
                // We succeeded at decrementing, so old_num is nonzero and we have at least one.
                // The lock prevents concurrent writers, so hand it out.
                return guard[old_num - 1];
            }
        }

        // If we haven't consumed the queue, do that now, then try again.
        if !has_consumed_queues {
            self.consume_incoming();
            return self.generate_id_impl(true);
        }

        // If we got here, we did our best. Generate a fresh one.
        self.next_never_used_id()
    }

    // Return an id to the mint.
    pub fn return_id(&self, id: ObjectId) {
        assert_eq!(id.get_ephemeral_bit(), self.ephemeral);
        // Only insert if we can increment the version. Otherwise, this id is
        // too old.
        if let Some(next) = id.next_version() {
            self.returned_ids.lock().unwrap().push(next);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mint() {
        let mint = Mint::new(false);
        let oids = (0..3).map(|_| mint.generate_id()).collect::<Vec<_>>();
        assert_eq!(
            oids,
            vec![
                ObjectId::new(0, 0, false),
                ObjectId::new(1, 0, false),
                ObjectId::new(2, 0, false)
            ]
        );
        for i in oids.into_iter() {
            mint.return_id(i);
        }
        let oids = (0..3).map(|_| mint.generate_id()).collect::<Vec<_>>();
        assert_eq!(
            oids,
            vec![
                ObjectId::new(2, 1, false),
                ObjectId::new(1, 1, false),
                ObjectId::new(0, 1, false)
            ]
        );
    }
}
