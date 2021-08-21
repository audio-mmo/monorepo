//! The [ObjectId] type.
//!
//! Objects are represented by a version, a unique identifier, and an ephemeral
//! bit.  The version and unique identifier make up the runtime identity of an
//! object, while the ephemeral bit is used for loading areas and similar into
//! the world.  Internally, the unique identifier is used as an index into a
//! variety of sets and maps, while the version is used to detect object reuse.
//!
//! It is only possible to have `2^32 - 1` unique objects; `u32::MAX` is
//! reserved as a sentinel for the future.

/// Mask to extract the ephemeral bit.
const EPHEMERAL_MASK: u32 = 1 << 31;

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct ObjectId {
    id: u32,
    // High bit of version is the ephemeral bit.
    version: u32,
}

impl ObjectId {
    pub(crate) fn new(id: u32, mut version: u32, ephemeral_bit: bool) -> ObjectId {
        debug_assert_ne!(id, u32::MAX);
        debug_assert_eq!(version & EPHEMERAL_MASK, 0);
        if ephemeral_bit {
            version |= EPHEMERAL_MASK;
        }
        Self { id, version }
    }

    pub fn get_version(&self) -> u32 {
        self.version & !EPHEMERAL_MASK
    }

    pub fn get_id(&self) -> u32 {
        self.id
    }

    pub fn get_ephemeral_bit(&self) -> bool {
        self.version & EPHEMERAL_MASK != 0
    }

    /// Returns whether this is the last time an object id of this version can be used because of wraparound in the version field.
    pub fn is_final_version(&self) -> bool {
        self.get_version() == u32::MAX >> 1
    }

    /// Generate a new `ObjectId` from this one with the same `id`.
    pub(crate) fn next_version(&self) -> Option<ObjectId> {
        if self.is_final_version() {
            None
        } else {
            Some(ObjectId::new(
                self.id,
                self.get_version() + 1,
                self.get_ephemeral_bit(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use proptest::prelude::*;

    #[test]
    fn simple_version_increment() {
        let oid = ObjectId::new(1, 1, false);
        let oid2 = oid.next_version().unwrap();
        assert_eq!(oid2.version, 2);
    }

    #[test]
    fn ephemeral_version_increment() {
        let oid = ObjectId::new(1, 1, true);
        assert_eq!(oid.version, (1 << 31) + 1);
        let oid2 = oid.next_version().unwrap();
        assert_eq!(oid2.version, (1 << 31) + 2);
    }

    proptest! {
        #[test]
        fn decomposing(
            id in 0..u32::MAX - 1,
            version in 0..u32::MAX/2,
            ephemeral_bit in prop::bool::ANY) {
                let oid = ObjectId::new(id, version, ephemeral_bit);
                prop_assert_eq!(oid.get_id(), id);
                prop_assert_eq!(oid.get_version(), version);
                prop_assert_eq!(oid.get_ephemeral_bit(), ephemeral_bit);
            }
    }

    #[test]
    fn version_increment_past_max() {
        let oid = ObjectId::new(1, u32::MAX / 2, false);
        assert_eq!(oid.next_version(), None);
        let oid2 = ObjectId::new(1, u32::MAX / 2, true);
        assert_eq!(oid2.next_version(), None);
    }
}
