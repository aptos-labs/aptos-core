// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Pipeline tests for `post` prefix in proof blocks.
module 0x42::TestPost {

    // ==========================================================================
    // Basic entry-point and return-point assertions.

    fun double(x: u64): u64 {
        x + x
    }
    spec double {
        requires x + x <= MAX_U64;
        ensures result == 2 * x;
    } proof {
        assert x + x == 2 * x;
        post assert result == x + x;
    }

    // ==========================================================================
    // Post with let: the let is visible in subsequent post statements.

    fun scale(x: u64, factor: u64): u64 {
        x * factor
    }
    spec scale {
        requires x * factor <= MAX_U64;
        ensures result == x * factor;
    } proof {
        let expected = x * factor;
        post assert result == expected;
    }

    // ==========================================================================
    // Post inside if/else branches.

    fun max(a: u64, b: u64): u64 {
        if (a >= b) { a } else { b }
    }
    spec max {
        ensures result >= a;
        ensures result >= b;
    } proof {
        if (a >= b) {
            post assert result == a;
        } else {
            post assert result == b;
        }
    }
}
