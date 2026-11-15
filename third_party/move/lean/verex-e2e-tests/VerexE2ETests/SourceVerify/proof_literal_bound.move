// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// A literal bound is matched up to its form: a nest of concatenations' premise
/// `<= MAX_U64` is stated by the sum below `MAX_U64 + 1`.
module 0x42::proof_literal_bound {
    fun stated(a: vector<u64>, b: vector<u64>, c: vector<u64>, half: u64, k: u64): vector<u64> {
        a
    }
    spec stated {
        requires k > 0;
        requires len(a) + len(b) + len(c) < half / k;
        ensures len(concat(concat(a, b), c)) == len(a) + len(b) + len(c);
    }
}
