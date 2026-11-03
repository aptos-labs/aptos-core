// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// The success side of `aborts_if k`: a returning path owes that no abort condition holds, keyed
/// by the clauses it owes, and a block takes one clause's conjunct.
module 0x42::proof_returns_clauses {
    fun rounded(x: u64, c: u64): u64 {
        x / c * c
    }
    spec rounded {
        pragma verify = true;
        requires c > 0;
        aborts_if x / c * c > x;
        ensures result <= x;
    }

    fun two(x: u64, c: u64): u64 {
        x / c * c
    }
    spec two {
        pragma verify = true;
        requires c > 0;
        aborts_if x / c * c > x;
        aborts_if c == 0;
        ensures result <= x;
    }
}
