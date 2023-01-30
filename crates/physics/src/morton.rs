//! An implementation of [Morton Coding](https://en.wikipedia.org/wiki/Z-order_curve).

/// A Morton-encoded pair of u16s, representing x/y coordinates.
pub struct MortonCode {
    /// Encoded as `y << 1 | x`
    data: u32,
}

/// Expand a u16 to `...-3-2-1-0` where `-` means unset bit.
const fn expand_u16(x: u16) -> u32 {
    // - - 2 1
    let mut res: u32 = x as u32;
    // - 2 - 1
    res = (res ^ (res << 8)) & 0x00ff00ff;
    // now we are working at the level of bits, but the pattern continues: shift left by a half, then use bitwise tricks
    // to zero out the ones we don't need anymore. Note that there are 0 bytes one byte to the left of any one byte.
    // Then 0 half-buytes to the left, etc.
    res = (res ^ (res << 4)) & 0x0f0f0f0f;
    res = (res ^ (res << 2)) & 0x33333333;
    (res ^ (res << 1)) & 0x55555555
}

/// Delete all of the odd bits of the given u32, pushing all even bits into a  u16.
fn collapse_u32(x: u32) -> u16 {
    let mut res = x & 0x55555555;
    res = (res ^ (res >> 1)) & 0x33333333;
    res = (res ^ (res >> 2)) & 0x0f0f0f0f;
    res = (res ^ (res >> 4)) & 0x00ff00ff;
    res = (res ^ (res >> 8)) & 0x0000ffff;
    res as u16
}

impl MortonCode {
    pub fn encode(x: u16, y: u16) -> MortonCode {
        MortonCode {
            data: expand_u16(x) | (expand_u16(y) << 1),
        }
    }

    /// Decode this mortonCode, returning `(x, y)`.
    pub fn decode(&self) -> (u16, u16) {
        (collapse_u32(self.data), collapse_u32(self.data >> 1))
    }

    /// Expand this morton code into two-bit pairs.
    ///
    /// Each pair is `yx` where the high bit is set if the high bit would have been set in y, and so on.  This is useful primarily as indices into quadtrees.
    fn as_quadrants(&self) -> [u8; 16] {
        let mut out: [u8; 16] = Default::default();

        for (i, dest) in out.iter_mut().enumerate() {
            let shift = 32 - i * 2 - 2;
            let mask = 3 << shift;
            *dest = ((self.data & mask) >> shift) as u8;
        }

        out
    }

    fn from_quadrants(quadrants: [u8; 16]) -> MortonCode {
        let data = quadrants
            .into_iter()
            .enumerate()
            .map(|(i, x)| {
                let shift = 32 - i * 2 - 2;
                (x as u32) << shift
            })
            .fold(0, |a, b| a | b);
        MortonCode { data }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest::proptest! {
        #[test]
        fn test_odd_bits_are_zero(x: u16) {
            assert_eq!(expand_u16(x) & 0xaaaaaaaa, 0);
        }
    }

    proptest::proptest! {
        #[test]
        fn test_expand_collapse_inverse(val: u16) {
            assert_eq!(collapse_u32(expand_u16(val)), val);
        }
    }

    proptest::proptest! {
        #[test]
        fn test_encode_decode_inverse(x: u16, y: u16) {
            let enc = MortonCode::encode(x, y);
            let (dec_x, dec_y) = enc.decode();
            assert_eq!((x, y), (dec_x, dec_y));
        }
    }

    proptest::proptest! {
        #[test]
        fn test_quadrants_inverses(x: u16, y: u16) {
            let code = MortonCode::encode(x, y);
            let quadrants = code.as_quadrants();
            let code2 = MortonCode::from_quadrants(quadrants);
            assert_eq!((x, y), code2.decode());
        }
    }

    fn boring_quadrant_computation(x: u16, y: u16) -> [u8; 16] {
        let mut out = [0; 16];
        for (i, dest) in out.iter_mut().enumerate() {
            let maskshift = 16 - i - 1;
            let mask = 1 << maskshift;
            let xbit = ((x & mask) != 0) as u8;
            let ybit = ((y & mask) != 0) as u8;
            *dest = (ybit << 1) | xbit;
        }

        out
    }

    proptest::proptest! {
        #[test]
        fn test_quadrants_against_boring(x: u16, y: u16) {
            let complicated = MortonCode::encode(x, y).as_quadrants();
            let boring = boring_quadrant_computation(x, y);
            assert_eq!(complicated, boring);
        }
    }
}
