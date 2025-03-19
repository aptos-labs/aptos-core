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
    // Binary search for the cut-off point where the bit at this depth turns from 0 to 1.
    // TODO: with stable partition_point: updates.partition_point(|&u| !u.0.bit(depth));
    let (mut i, mut j) = (0, updates.len());
    // Find the first index that starts with bit 1.
    while i < j {
        let mid = i + (j - i) / 2;
        if updates[mid].0.hash_ref().bit(depth) {
            j = mid;
        } else {
            i = mid + 1;
        }
    }
    i
}
