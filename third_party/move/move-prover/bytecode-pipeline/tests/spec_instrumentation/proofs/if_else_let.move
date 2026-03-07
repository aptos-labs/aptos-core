// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Pipeline tests for if/else and let proof constructs.
module 0x42::TestIfElseLet {

    // ==========================================================================
    // If/else in proof block.

    fun test_if_else(x: u64): u64 {
        if (x > 10) { x } else { 10 }
    }
    spec test_if_else {
        ensures result >= 10;
    } proof {
        if (x > 10) {
            assert x >= 10;
        } else {
            assert 10 >= 10;
        }
    }

    // ==========================================================================
    // Let binding in proof block.

    fun test_let(x: u64): u64 {
        x + 1
    }
    spec test_let {
        ensures result == x + 1;
    } proof {
        let y = x + 1;
        post assert y == result;
    }
}
