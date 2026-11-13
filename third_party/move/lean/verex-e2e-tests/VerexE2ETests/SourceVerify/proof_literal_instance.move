// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// An instance of a quantified fact at a literal position.
module 0x42::proof_literal_instance {
    fun first(v: vector<i64>): vector<i64> {
        v
    }
    spec first {
        requires len(v) > 0;
        requires forall k in 0..len(v): v[k] >= 0;
        ensures v[0] >= 0;
    }
}
