// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// A concatenation's size is linear in its parts' sizes.
module 0x42::proof_concat_size {
    fun sizes(a: vector<u64>, b: vector<u64>, i: u64): vector<u64> {
        a
    }
    spec sizes {
        requires len(a) + len(b) < MAX_U64;
        requires i < len(b);
        ensures i + len(a) < len(concat(a, b));
    }
}
