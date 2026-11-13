// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// `after call k of g` gives its facts to what follows the call: here what the
/// returning path owes of the abort conditions, keyed by its clause.
module 0x42::proof_after_key {
    fun same(x: u64): u64 {
        x
    }
    spec same {
        pragma opaque;
        aborts_if false;
        ensures result == x;
    }

    fun caller(x: u64, c: u64): u64 {
        same(x)
    }
    spec caller {
        requires c > 0;
        aborts_if x / c * c > x;
    }
}
