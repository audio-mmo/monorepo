//! The [Mint] produces new object ids, either from a deterministic sequence or from a random number generator.
//!
//! The deterministic mode can be used in tests.
use std::sync::Mutex;

use crate::object_id::ObjectId;

/// Hidden interior state of the mint, because if the enum is pub then so are the variants.
#[derive(Debug)]
enum MintInner {
    /// Get the id from a random number generator.
    Random,
    /// Get the id from a  seeded random number generator.
    SeededRandom(Box<Mutex<rand_chacha::ChaChaRng>>),
    Deterministic(Mutex<u64>),
}

impl MintInner {
    fn next(&self, ephemeral: bool) -> ObjectId {
        use rand::prelude::*;

        use MintInner::*;

        let (upper, lower) = match self {
            Random => {
                let mut rng = rand::thread_rng();
                (rng.gen::<u64>(), rng.gen::<u64>())
            }
            SeededRandom(r) => {
                let mut g = r.lock().unwrap();
                (g.gen::<u64>(), g.gen::<u64>())
            }
            Deterministic(counter) => {
                let mut guard = counter.lock().unwrap();
                // Increment first, thus never generating 0.
                *guard += 1;
                (0, *guard)
            }
        };

        ObjectId::new(ephemeral, upper, lower)
    }
}

#[derive(Debug)]
pub struct Mint {
    ephemeral: bool,
    inner: MintInner,
}

impl Mint {
    /// generate a new non-deterministic mint.  Returns concrete object ids.
    pub fn new() -> Mint {
        Self::new_ephemeral(false)
    }

    /// A mint that generates ids with the specified ephemeralness.
    pub fn new_ephemeral(ephemeral: bool) -> Mint {
        Mint {
            ephemeral,
            inner: MintInner::Random,
        }
    }

    /// A mint which generates ids in a deterministic fashion.
    pub fn new_deterministic(ephemeral: bool) -> Mint {
        Mint {
            ephemeral,
            inner: MintInner::Deterministic(Mutex::new(0)),
        }
    }

    pub fn new_seeded(seed: [u8; 32], ephemeral: bool) -> Mint {
        use rand::prelude::*;

        Mint {
            ephemeral,
            inner: MintInner::SeededRandom(Box::new(Mutex::new(
                rand_chacha::ChaCha20Rng::from_seed(seed),
            ))),
        }
    }

    pub fn next(&self) -> ObjectId {
        self.inner.next(self.ephemeral)
    }
}

impl Default for Mint {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn random_mints_no_duplicate() {
        let mint = Mint::default();
        let mut s = HashSet::new();
        for _ in 0..1000 {
            s.insert(mint.next());
        }
        assert_eq!(s.len(), 1000);
    }

    #[test]
    fn respects_ephemeral() {
        let m1 = Mint::new_ephemeral(true);
        let i1 = m1.next();
        assert!(i1.get_ephemeral_bit());
        let m2 = Mint::new_deterministic(true);
        let i2 = m2.next();
        assert!(i2.get_ephemeral_bit());
        let m3 = Mint::new_seeded([1; 32], true);
        let i3 = m3.next();
        assert!(i3.get_ephemeral_bit());
    }

    // Let's stop someone changing the rng without realising.
    #[test]
    fn seeded_determinism() {
        let mut seed = [0; 32];
        (0..32u8).for_each(|i| seed[i as usize] = i);
        let mint = Mint::new_seeded(seed, false);
        let mut items = vec![];
        for _ in 0..5 {
            items.push(mint.next());
        }
        assert_eq!(
            items,
            vec![
                ObjectId::new(false, 7645359380336737593, 5281276197874154893),
                ObjectId::new(false, 5506458395325511050, 10530800043416210610),
                ObjectId::new(false, 3108434420605657899, 7241726879045979711),
                ObjectId::new(false, 3288744496421241381, 883087369427888066),
                ObjectId::new(false, 5883643594736318488, 2832275636194402579)
            ]
        );
    }
}
