// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::sparse_merkle::HashValueRef;
use aptos_crypto::HashValue;

/// Returns the shard id of the hashed key.
pub fn get_state_shard_id(key: &HashValue) -> u8 {
    key.nibble(0)
}

/// Swap template-type values if 'cond'=true - useful to determine left/right parameters.
pub(crate) fn swap_if<T>(first: T, second: T, cond: bool) -> (T, T) {
    if cond {
        (second, first)
    } else {
        (first, second)
    }
}

/// Return the index of the first bit that is 1 at the given depth when updates are
/// lexicographically sorted.
pub(crate) fn partition<T>(updates: &[(impl HashValueRef, T)], depth: usize) -> usize {
    if let Some(first) = updates.first() {
        updates.iter().skip(1).for_each(|u| {
            debug_assert!(
                u.0.hash_ref().common_prefix_bits_len(*first.0.hash_ref()) >= depth,
                "The first {depth} bits must be the same."
            );
        });
    }
    // Find the first index that starts with bit 1.
    updates.partition_point(|u| !u.0.hash_ref().bit(depth))
}
