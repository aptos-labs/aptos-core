// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Pipeline tests for `split` proof statement.
module 0x42::TestSplit {

    // ==========================================================================
    // Split on a boolean parameter.

    fun abs_diff(a: u64, b: u64): u64 {
        if (a >= b) { a - b } else { b - a }
    }
    spec abs_diff {
        ensures result == if (a >= b) { a - b } else { b - a };
    } proof {
        split a >= b;
    }

    // ==========================================================================
    // Split on an enum.

    enum Color has drop {
        Red,
        Green,
        Blue,
    }

    fun color_code(c: Color): u64 {
        match (c) {
            Color::Red => 1,
            Color::Green => 2,
            Color::Blue => 3,
        }
    }
    spec color_code {
        ensures result >= 1;
        ensures result <= 3;
    } proof {
        split c;
    }
}
