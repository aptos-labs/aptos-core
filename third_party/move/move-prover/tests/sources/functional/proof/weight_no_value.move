// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// flag: --language-version=2.4

module 0x42::proof_weight_no_value {

    // weight = 1000 is far above the eager_threshold (100). Z3 defers this
    // axiom immediately, so the equation `id_num(0) == 0` never propagates.
    spec fun id_num(n: num): num {
        if (n == 0) { 0 } else { id_num(n - 1) + 1 }
    }

    fun id_zero(): u64 {
        0
    }
    spec id_zero {
        // Even this trivial post-condition can't discharge: the prover
        // never gets to unfold `id_num(0)`.
        ensures result == id_num(0);
    }
}
