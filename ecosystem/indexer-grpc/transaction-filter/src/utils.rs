// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// 64 "0"s
const ZEROS: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Converts a "0x" prefixed address to its LONG format
#[inline]
pub fn standardize_address(address: &str) -> String {
    // Preallocate exactly the required capacity.
    let trimmed = &address[2..];
    let mut result = String::with_capacity(66);

    // Push the appropriate slice of zeros rather than iterating.
    result.push_str("0x");
    result.push_str(&ZEROS[..64 - trimmed.len()]);
    result.push_str(trimmed);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standardize_address() {
        assert_eq!(
            standardize_address("0x1"),
            "0x0000000000000000000000000000000000000000000000000000000000000001"
        );
    }

    #[test]
    fn test_standardize_medium_length_address() {
        assert_eq!(
            standardize_address("0x123abc"),
            "0x0000000000000000000000000000000000000000000000000000000000123abc"
        );
    }

    #[test]
    fn test_standardize_full_length_address() {
        assert_eq!(
            standardize_address(
                "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
            ),
            "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
        );
    }

    #[test]
    fn test_standardize_address_with_leading_zero() {
        assert_eq!(
            standardize_address(
                "0x234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
            ),
            "0x0234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
        );
    }
}
