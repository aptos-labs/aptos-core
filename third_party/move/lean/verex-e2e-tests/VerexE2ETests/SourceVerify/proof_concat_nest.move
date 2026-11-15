// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// A nest of concatenations is one concatenation under one premise, the sum of its leaves'
/// lengths in their order; a proof states that one bound.
module 0x42::proof_concat_nest {
    fun stated(a: vector<u64>, b: vector<u64>, c: vector<u64>, d: vector<u64>, half: u64, k: u64): vector<u64> {
        a
    }
    spec stated {
        requires k > 0;
        requires len(a) + len(b) + len(c) + len(d) < half / k;
        ensures len(concat(concat(concat(a, b), c), d)) == len(a) + len(b) + len(c) + len(d);
    }
}
