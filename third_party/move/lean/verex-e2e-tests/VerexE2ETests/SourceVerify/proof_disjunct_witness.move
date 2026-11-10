// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// A disjunction obligation closed by a stated disjunct: the existential is
/// stated with its witness, the disjunction follows.
module 0x42::proof_disjunct_witness {
    fun bad(v: vector<u64>, i: u64, flag: bool): vector<u64> {
        v
    }
    spec bad {
        requires i > 0;
        requires i < len(v);
        requires v[i - 1] >= v[i];
        ensures (exists k: u64: k < len(v) - 1 && v[k] >= v[k + 1]) || flag || len(v) == 0;
    }
}
