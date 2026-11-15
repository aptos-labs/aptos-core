// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// A fact under `if` is an implication and no rule of the block's normal form:
/// it does not rewrite the length out of the disjunct `decide` gives, which
/// closes the goal, in a context that arithmetic refutes.
module 0x42::proof_implication_facts {
    fun at(v: vector<u64>, i: u64, r: u64, a: u64, b: u64, s: u64, e: u64, n: u64): u64 {
        v[r]
    }
    spec at {
        requires a * s + r == i;
        requires i < e;
        requires n > 0;
        requires n == 0 || b * s + len(v) == e;
        requires a <= b && b <= a;
        aborts_if false;
    }
}
