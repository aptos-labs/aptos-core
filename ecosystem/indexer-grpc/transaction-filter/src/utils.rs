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
