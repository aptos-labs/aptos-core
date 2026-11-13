// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// `decide` reads a disjunction whose other disjunct is refuted.
module 0x42::proof_disjuncts {
    fun positive_at(v: vector<u64>, n: u64, i: u64): vector<u64> {
        v
    }
    spec positive_at {
        requires n == 0 || (forall k in 0..n: v[k] > 0);
        requires i < n;
        ensures v[i] > 0;
    }
}
