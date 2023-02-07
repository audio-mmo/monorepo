//! An implementation of [Morton Coding](https://en.wikipedia.org/wiki/Z-order_curve).
use proptest::strategy::Strategy;

/// A Morton-encoded pair of u16s, representing x/y coordinates.
#[derive(Copy, Clone, Debug, Eq, PartialEq, derive_more::Display)]
#[display(fmt=:"{:x}", code)]
pub struct MortonCode {
    /// Encoded as `y << 1 | x`
    data: u32,
}

/// A morton prefix combines two morton codes, and represents the common prefix between them.
///
/// For example, if the morton codes are the corners of a box, their prefix is the path in a quadtree which leads to the
/// smallest node completely containing that box.
///
/// Our prefixes are arrays of (0..4) values, where the high bit is set if the y bit in the morton code is nonzero, and
/// the low bit is set if the x coordinate is nonzero.  We never work in or return just one bit.
#[derive(Copy, Clone, Debug, proptest_derive::Arbitrary)]
pub struct MortonPrefix {
    code: u32,

    /// Index of the first common bit in the integer, starting from the least significant. AN empty prefix is 32.  A full prefix is 0.

    // the strategy here is reversed with prop_flat_map because it must shrink toward 32.
    #[proptest(strategy = "(0u8..32).prop_map(|x| 32 - x)")]
    first_valid_bit: u8,
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

impl MortonPrefix {
    pub fn from_code(code: MortonCode) -> MortonPrefix {
        MortonPrefix {
            code: code.data,
            // Right now, it's the whole thing.
            first_valid_bit: 0,
        }
    }

    /// Merge this prefix with another prefix, producing the prefix which is the prefix of both prefixes.
    #[must_use = "This returns a new prefix"]
    pub fn merge(&self, other: MortonPrefix) -> MortonPrefix {
        // We want the first varying bit from the top, which we then invert into the first equal bit from the bottom.
        let xored = self.code ^ other.code;
        // First bit which varies, from the top.  Note that trailing zeros is incorrect: we know only that the prefixes
        // are the same, but the low bits may have more than one varying bit.
        let top_varying = xored.leading_zeros();
        // So flip it: 32 - top_varying is the index of the lowest bit which varies, 0 if no bits were set.
        let bottom_varying = 32 - top_varying;
        // the actual thing we want is the maximum of all 3, the highest bit which is the same accounting for already valid parts of the prefixes.
        let candidate = bottom_varying
            .max(self.first_valid_bit as u32)
            .max(other.first_valid_bit as u32);
        // We now bump candidate up to the next multiple of 2, since we never deal in fractional bits.
        let actual = (candidate + 1) & (!1);
        MortonPrefix {
            code: self.code,
            first_valid_bit: actual.try_into().unwrap(),
        }
    }

    /// Unpack this prefix into an iterator of `u8` of the form `yx` where y and x aree bits whicha re set if the
    /// appropriate bit is set in the prefix.
    pub fn unpack(&self) -> impl Iterator<Item = u8> {
        let num_elems = (32 - self.first_valid_bit) / 2;
        let mask = 0xc0000000;
        let mut code = self.code;
        (0..num_elems).map(move |_| {
            let item = (code & mask) >> 30;
            code <<= 2;
            item.try_into().unwrap()
        })
    }
}

impl std::cmp::PartialEq for MortonPrefix {
    fn eq(&self, other: &Self) -> bool {
        if self.first_valid_bit != other.first_valid_bit {
            return false;
        }

        // We need to zero out the lower bits, which are intentionally left to have any arbitrary value.
        let mask = u32::MAX << self.first_valid_bit;
        (self.code & mask) != (other.code & mask)
    }
}

impl std::cmp::Eq for MortonPrefix {}

impl std::fmt::Display for MortonPrefix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:x}/{}", self.code, 32 - self.first_valid_bit)
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

    #[track_caller]
    fn test_prefix((x1, y1): (u16, u16), (x2, y2): (u16, u16), expected: &[u8]) {
        let code1 = MortonCode::encode(x1, y1);
        let code2 = MortonCode::encode(x2, y2);
        let prefix = MortonPrefix::from_code(code1).merge(MortonPrefix::from_code(code2));
        let got = prefix.unpack().collect::<Vec<_>>();
        assert_eq!(&got[..], expected);
    }

    #[test]
    fn test_prefixes_basic() {
        assert_eq!(
            MortonPrefix::from_code(MortonCode::encode(0xffff, 0xffff))
                .unpack()
                .collect::<Vec<_>>(),
            vec![3; 16]
        );

        test_prefix(
            (0xffff, 0xffff),
            (0xffff, 0xffff),
            &[3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3],
        );

        test_prefix(
            (0xff00, 0xff00),
            (0xffff, 0xffff),
            &[3, 3, 3, 3, 3, 3, 3, 3],
        );

        test_prefix((0xffff, 0xffff), (0, 0), &[]);
    }

    fn boring_morton_prefix((x1, y1): (u16, u16), (x2, y2): (u16, u16)) -> Vec<u8> {
        let m1 = MortonCode::encode(x1, y1).as_quadrants();
        let m2 = MortonCode::encode(x2, y2).as_quadrants();
        m1.into_iter()
            .zip(m2.into_iter())
            .take_while(|(x, y)| x == y)
            .map(|i| i.0)
            .collect()
    }

    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig {
            cases: 10000000,
            ..Default::default()
        })]
        #[test]
        fn test_prefixes_fuzz(
            x1: u16,
            y1: u16,
            x2: u16,
            y2: u16,
        ) {
            let p1 = MortonPrefix::from_code(MortonCode::encode(x1, y1));
            let p2 = MortonPrefix::from_code(MortonCode::encode(x2, y2));
            let merged = p1.merge(p2);
            let unpacked = merged.unpack().collect::<Vec<_>>();
            let expected = boring_morton_prefix((x1, y1), (x2, y2));
            assert_eq!(unpacked, expected);
        }
    }
}
