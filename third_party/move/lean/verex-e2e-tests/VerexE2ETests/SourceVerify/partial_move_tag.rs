// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Rustc reads the discriminant again after moving the Ok payload to decide
// whether the Err payload still needs dropping. That read must not copy the
// partially moved enum.
pub fn take(value: Result<Vec<u8>, Vec<u8>>) -> Option<Vec<u8>> {
    match value {
        Ok(bytes) => Some(bytes),
        Err(_) => None,
    }
}
