// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Pipeline tests for the induct on proof hint.
module 0x42::TestInduct {

    // ==========================================================================
    // Induction on a u64 parameter → creates base/step variants.

    fun test_induct(n: u64): u64 {
        n
    }
    spec test_induct {
        ensures result == n;

        proof {
            induct on n;
        }
    }
}
