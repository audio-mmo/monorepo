//! The [ObjectId] type.
//!
//! Objects are represented by a 127-bit uniformly distributed random id, and a
//! 1-bit ephemeral flag.  This is then split across 2 u64 fields, because while
//! Rust supports 128-bit integers, these ids are their own 64-bit hashes.
//!
//! Layout is `(ephemeral, upper, lower)` where ephemeral is the most
//! significant bit of upper.
//!
//! Additionally upper is a `NonZeroU64` so that this can work well with
//! `Option`.  To make that work we specifically check for it and replace it
//! with 1, removing exactly 1 value from the 127 bits of randomness.
//!
//! This is an odd design for an ECS, but is justified by 3 factors: first, we
//! don't need to be as fast as other implementations (e.g. no "let's do
//! particle systems where every particle is an entity").  Second, we need
//! unique ids across the network.  Third, we nee unique ids across potentially
//! realtime years.  Let's kill the complexity of versions and kill the
//! complexity of some auxiliary system to reference objects and just take the
//! slight performance hit.
use std::num::NonZeroU64;

/// Mask to extract the ephemeral bit.
const EPHEMERAL_MASK: u64 = 1 << 63;

#[derive(Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct ObjectId {
    upper: NonZeroU64,
    lower: u64,
}

impl ObjectId {
    pub(crate) fn new(ephemeral: bool, mut upper: u64, lower: u64) -> ObjectId {
        upper &= !EPHEMERAL_MASK;
        upper |= (ephemeral as u64) << 63;
        if upper == 0 {
            upper = 1;
        }
        unsafe {
            Self {
                upper: NonZeroU64::new_unchecked(upper),
                lower,
            }
        }
    }

    pub fn get_ephemeral_bit(&self) -> bool {
        self.upper.get() & EPHEMERAL_MASK != 0
    }
}

impl std::fmt::Debug for ObjectId {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.debug_struct("ObjectId")
            .field("ephemeral", &self.get_ephemeral_bit())
            .field("upper", &(self.upper.get() & !EPHEMERAL_MASK))
            .field("lower", &self.lower)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ephemeral() {
        let not_ephemeral = ObjectId::new(false, 1, 2);
        assert_eq!(not_ephemeral.upper.get(), 1);
        assert_eq!(not_ephemeral.lower, 2);
        let ephemeral = ObjectId::new(true, 1, 2);
        assert_eq!(ephemeral.upper.get(), 0x8000000000000001);
        assert_eq!(ephemeral.lower, 2);
    }

    #[test]
    fn no_accidental_ephemeral() {
        let oid = ObjectId::new(false, 1 << 63, 2);
        assert_eq!(oid.upper.get(), 1);
        assert_eq!(oid.lower, 2);
    }
}
