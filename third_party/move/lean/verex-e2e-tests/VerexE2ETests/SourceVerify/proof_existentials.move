// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// A fact stated as an existential is taken apart, and `let x : T where φ`
/// names the witness, from a hypothesis and from an instance of a guarded
/// quantified fact.
module 0x42::proof_existentials {
    fun even(x: u64): u64 {
        x
    }
    spec even {
        requires exists half: u64: x == 2 * half;
        ensures result % 2 == 0;
    }

    fun evens(v: vector<u64>, i: u64): vector<u64> {
        v
    }
    spec evens {
        requires forall k in 0..len(v): exists half: u64: v[k] == 2 * half;
        requires i < len(v);
        ensures v[i] % 2 == 0;
    }

    fun stated(v: vector<u64>, i: u64): vector<u64> {
        v
    }
    spec stated {
        requires forall k in 0..len(v): exists half: u64: v[k] == 2 * half;
        requires i < len(v);
        ensures v[i] % 2 == 0;
    }
}
