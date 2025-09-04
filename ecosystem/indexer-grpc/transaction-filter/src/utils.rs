// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

// 64 "0"s
const ZEROS: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Converts a "0x" prefixed address to display format (short for special addresses, long for all other addresses):
/// https://github.com/velor-foundation/AIPs/blob/main/aips/aip-40.md#display-format
#[inline]
pub fn standardize_address(address: &str) -> String {
    // Remove "0x" prefix if it exists
    let trimmed = address.strip_prefix("0x").unwrap_or(address);

    // Check if the address is a special address by seeing if the first 31 bytes are zero and the last byte is smaller than 0b10000
    if let Some(last_char) = trimmed.chars().last() {
        if trimmed[..trimmed.len().saturating_sub(1)]
            .chars()
            .all(|c| c == '0')
            && last_char.is_ascii_hexdigit()
            && last_char <= 'f'
        {
            // Return special addresses in short format
            let mut result = String::with_capacity(3);
            result.push_str("0x");
            result.push(last_char);
            return result;
        }
    }

    // Return non-special addresses in long format
    let mut result = String::with_capacity(66);
    result.push_str("0x");
    result.push_str(&ZEROS[..64 - trimmed.len()]);
    result.push_str(trimmed);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standardize_special_address() {
        assert_eq!(standardize_address("0x1"), "0x1");
        assert_eq!(standardize_address("0x01"), "0x1");
        assert_eq!(standardize_address("0x001"), "0x1");
        assert_eq!(standardize_address("0x000000001"), "0x1");
        assert_eq!(standardize_address("0xf"), "0xf");
        assert_eq!(standardize_address("0x0f"), "0xf");
        assert_eq!(
            standardize_address(
                "0x0000000000000000000000000000000000000000000000000000000000000001"
            ),
            "0x1"
        );

        assert_eq!(standardize_address("1"), "0x1");
        assert_eq!(
            standardize_address("0000000000000000000000000000000000000000000000000000000000000001"),
            "0x1"
        );
    }

    #[test]
    fn test_standardize_not_special_address() {
        assert_eq!(
            standardize_address("0x10"),
            "0x0000000000000000000000000000000000000000000000000000000000000010"
        );

        assert_eq!(
            standardize_address("10"),
            "0x0000000000000000000000000000000000000000000000000000000000000010"
        );

        assert_eq!(
            standardize_address("0x123abc"),
            "0x0000000000000000000000000000000000000000000000000000000000123abc"
        );

        assert_eq!(
            standardize_address(
                "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
            ),
            "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
        );
    }

    #[test]
    fn test_standardize_address_with_missing_leading_zero() {
        assert_eq!(
            standardize_address(
                "0x234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
            ),
            "0x0234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
        );

        assert_eq!(
            standardize_address("234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"),
            "0x0234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
        );
    }
}
