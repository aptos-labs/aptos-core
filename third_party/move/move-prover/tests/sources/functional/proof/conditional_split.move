// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// flag: --language-version=2.4

// E2E tests for `split` nested under `if` in proof blocks.
module 0x42::proof_conditional_split {

    // ==================================================================
    // Boolean split under an if guard.

    fun abs_max(a: u64, b: u64): u64 {
        if (a == 0 && b == 0) { 0 }
        else if (a >= b) { a }
        else { b }
    }
    spec abs_max {
        ensures result >= a;
        ensures result >= b;
    } proof {
        if (a > 0 || b > 0) {
            split a >= b;
        }
    }

    // ==================================================================
    // Enum split under an if guard.

    enum Color has drop {
        Red,
        Green,
        Blue,
    }

    fun color_or_default(c: Color, use_default: bool): u64 {
        if (use_default) { 0 }
        else {
            match (c) {
                Color::Red => 1,
                Color::Green => 2,
                Color::Blue => 3,
            }
        }
    }
    spec color_or_default {
        ensures result <= 3;
    } proof {
        if (!use_default) {
            split c;
        }
    }
}
