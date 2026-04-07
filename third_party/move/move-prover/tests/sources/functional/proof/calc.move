// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// flag: --language-version=2.4

// E2E tests for `calc(...)` in proof blocks.
module 0x42::proof_calc {

    // ==================================================================
    // Multi-step equality chain: show that a composed arithmetic
    // expression simplifies step by step.

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

    // ==================================================================
    // Calc chain mixing == and <= operators: show a division result
    // is bounded step by step.

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

    // ==================================================================
    // Multi-step calc with intermediate >= steps, proving a lower
    // bound through a chain of inequalities.

    fun double_plus_one(x: u64): u64 {
        2 * x + 1
    }
    spec double_plus_one {
        requires 2 * x + 1 <= MAX_U64;
        ensures result >= x;
    } proof {
        calc(
            2 * x + 1
            >= 2 * x
            >= x
        );
    }

    // ==================================================================
    // FAILURE: Wrong calc step.
    // The step `x + 1 + 1 == x + 3` is off by one.

    fun add_two(x: u64): u64 {
        x + 1 + 1
    }
    spec add_two {
        requires x + 2 <= MAX_U64;
        ensures result == x + 2;
    } proof {
        calc(
            x + 1 + 1
            == x + 3  // error: should be x + 2
        );
    }
}
