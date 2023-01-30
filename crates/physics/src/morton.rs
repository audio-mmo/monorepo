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
}
