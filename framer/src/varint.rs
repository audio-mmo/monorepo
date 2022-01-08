use bytes::{Buf, BufMut};

/// Number of bytes to completely encode u64.
pub(crate) const MAX_BYTES: u64 = 10;

#[derive(Debug, Eq, PartialEq, thiserror::Error, derive_more::Display)]
pub(crate) enum VarintError {
    /// The input buffer was empty.
    NoData,

    /// The varint was too long to be a u64.
    TooLong,

    /// The varint isn't complete yet.  Contains the partial value.
    Incomplete(u64),
}

/// A simple varint encoder.
///
/// Writes in little endian order.
pub(crate) fn encode_varint(mut input: u64, dest: &mut impl BufMut) {
    loop {
        let nb = input & 0x7f;
        debug_assert_eq!(nb, nb as u8 as u64);
        input >>= 7;
        if input == 0 {
            dest.put_u8(nb as u8);
            break;
        }
        dest.put_u8((nb | 0x80) as u8);
    }
}

/// A simple varint decoder.
///
/// Assumes that the varint is little endian, and no more than u64::MAX.  Returns `IncompleteVarint` with the partial value so far as an error in the case of incomplete varints, otherwise
pub(crate) fn decode_varint(input: &mut impl Buf) -> Result<u64, VarintError> {
    let mut res = 0;

    for i in 0..MAX_BYTES {
        if !input.has_remaining() {
            if i == 0 {
                return Err(VarintError::NoData);
            }
            return Err(VarintError::Incomplete(res));
        }

        let byte = input.get_u8();
        let done = (byte & 0x80) == 0;
        let val = byte & 0x7f;
        res += (val as u64) << (7 * i);

        if done {
            return Ok(res);
        }
    }

    // If we got here, we didn't finish.
    Err(VarintError::TooLong)
}

#[cfg(test)]
mod tests {
    use super::*;

    use proptest::prelude::*;

    /// Assert that an input value can round-trip through a vec and back.
    fn check_roundtrip(input: u64) {
        let mut buf = vec![];
        encode_varint(input, &mut buf);
        let output = decode_varint(&mut &buf[..]).expect("Should decode");
        assert_eq!(input, output, "{:?}", buf);
    }

    #[test]
    fn test_basic_roundtrips() {
        check_roundtrip(0);
        check_roundtrip(u64::MAX);
        check_roundtrip(u32::MAX as u64);
        check_roundtrip(u16::MAX as u64);
        check_roundtrip(u8::MAX as u64);
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100000))]
        #[test]
        fn test_fuzz(val: u64) {
            check_roundtrip(val);
        }
    }

    #[test]
    fn test_encoding_error_conditions() {
        assert_eq!(decode_varint(&mut &vec![][..]), Err(VarintError::NoData));
        assert_eq!(
            decode_varint(&mut &vec![0x80, 0xff][..]),
            Err(VarintError::Incomplete(0b11111110000000))
        );
    }
}
