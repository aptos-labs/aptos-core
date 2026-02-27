// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// flag: --language-version=2.4

// E2E tests for `split` in proof blocks.
module 0x42::proof_split {

    // Split on a boolean expression.
    fun abs_diff(a: u64, b: u64): u64 {
        if (a >= b) { a - b } else { b - a }
    }
    spec abs_diff {
        ensures result == if (a >= b) { a - b } else { b - a };
    } proof {
        split a >= b;
    }

    // Split on a boolean with a non-trivial condition.
    fun max(a: u64, b: u64): u64 {
        if (a >= b) { a } else { b }
    }
    spec max {
        ensures result >= a;
        ensures result >= b;
    } proof {
        split a >= b;
    }

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

    // ==================================================================
    // FAILURE: Split doesn't prove a too-strong ensures.
    // The ensures claims result >= 10, which is false when x < 10.

    fun clamp_above(x: u64): u64 {
        if (x >= 5) { x } else { 5 }
    }
    spec clamp_above {
        ensures result >= 10;  // too strong: result can be 5
    } proof {
        split x >= 5;
    }
}
