// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// Negated quantifiers and connectives in negation normal form: the negated
/// existential is a universal fact whose instance states the conclusion.
module 0x42::proof_negations {
    fun sorted_pair(v: vector<u64>, x: u64): vector<u64> {
        v
    }
    spec sorted_pair {
        requires x + 1 < len(v);
        requires !(exists k: num: 0 <= k && k + 1 < len(v) && v[k] >= v[k + 1]);
        ensures v[x] < v[x + 1];
    }
}
