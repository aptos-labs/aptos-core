// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::bail;

pub fn serialize_uleb128(buffer: &mut Vec<u8>, mut val: u64) -> anyhow::Result<()> {
    loop {
        let cur = val & 0x7F;
        if cur != val {
            buffer.push((cur | 0x80) as u8);
            val >>= 7;
        } else {
            buffer.push(cur as u8);
            break;
        }
    }
    Ok(())
}

/// Returns Uleb128 value, followed by bytes read
pub fn deserialize_uleb128(buffer: &[u8]) -> anyhow::Result<(u64, usize)> {
    let mut value: u64 = 0;
    let mut shift = 0;
    let mut i = 0;

    // Go through values until the full number is found
    loop {
        let byte = buffer[i];
        let cur = (byte & 0x7F) as u64;
        if (cur << shift) >> shift != cur {
            bail!("invalid ULEB128 repr for usize");
        }
        value |= cur << shift;

        if (byte & 0x80) == 0 {
            if shift > 0 && cur == 0 {
                bail!("invalid ULEB128 repr for usize");
            }
            return Ok((value, i + 1));
        }

        shift += 7;
        if shift > u64::BITS {
            break;
        }
        i += 1;
    }
    Err(anyhow::Error::msg("invalid ULEB128 repr for usize"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize() {
        test_serialize_case(0, "00");
        test_serialize_case(1, "01");
        test_serialize_case(2, "02");
        test_serialize_case(3, "03");
        test_serialize_case(4, "04");
        test_serialize_case(5, "05");
        test_serialize_case(6, "06");
        test_serialize_case(7, "07");
        test_serialize_case(8, "08");
        test_serialize_case(9, "09");
        test_serialize_case(10, "0A");
        test_serialize_case(11, "0B");
        test_serialize_case(12, "0C");
        test_serialize_case(13, "0D");
        test_serialize_case(14, "0E");
        test_serialize_case(15, "0F");
        test_serialize_case(16, "10");
        test_serialize_case(17, "11");
        test_serialize_case(624485, "E58E26");
        test_serialize_case(u64::MAX, "FFFFFFFFFFFFFFFFFF01");
    }

    #[test]
    fn test_deserialize() {
        test_deserialize_case("00", 0, 1);
        test_deserialize_case("01", 1, 1);
        test_deserialize_case("E58E26", 624485, 3);
        test_deserialize_case("FFFFFFFFFFFFFFFFFF01", u64::MAX, 10);
    }

    fn test_serialize_case(value: u64, expected: &str) {
        let mut buffer = vec![];
        serialize_uleb128(&mut buffer, value).expect("Should not fail");
        assert_eq!(buffer, hex::decode(expected).unwrap())
    }

    fn test_deserialize_case(bytes: &str, expected: u64, expected_num_bytes: usize) {
        let buffer = hex::decode(bytes).unwrap();
        let (value, num_bytes) = deserialize_uleb128(&buffer).unwrap();

        assert_eq!(num_bytes, expected_num_bytes);
        assert_eq!(value, expected);
    }
}
