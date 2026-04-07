// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Pipeline tests for `calc(...)` in proof blocks.
module 0x42::TestCalc {

    // ==========================================================================
    // Multi-step equality chain.

    fun add_three(x: u64): u64 {
        x + 1 + 1 + 1
    }
    spec add_three {
        requires x + 3 <= MAX_U64;
        ensures result == x + 3;
    } proof {
        calc(
            x + 1 + 1 + 1
            == x + 2 + 1
            == x + 3
        );
    }

    // ==========================================================================
    // Calc chain with inequality operators.

    fun half(x: u64): u64 {
        x / 2
    }
    spec half {
        ensures result <= x;
    } proof {
        calc(
            x / 2
            <= x
        );
    }
}
