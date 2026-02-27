// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Pipeline tests for the `split on` proof hint (bool and enum).
module 0x42::TestSplit {

    // ==========================================================================
    // Bool split: true/false cases.

    fun test_split_bool(a: u64, b: u64): u64 {
        if (a >= b) { a - b } else { b - a }
    }
    spec test_split_bool {
        ensures result == if (a >= b) { a - b } else { b - a };

        proof {
            split on a >= b;
        }
    }

    // ==========================================================================
    // Enum split with 3 variants.

    enum Color has drop { Red, Green, Blue }

    fun color_code(c: Color): u64 {
        match (c) {
            Color::Red => 1,
            Color::Green => 2,
            Color::Blue => 3,
        }
    }
    spec color_code {
        ensures result >= 1 && result <= 3;

        proof {
            split on c;
        }
    }

    // ==========================================================================
    // Enum split with 2 variants.

    enum Toggle has drop { On, Off }

    fun is_on(t: Toggle): bool {
        match (t) {
            Toggle::On => true,
            Toggle::Off => false,
        }
    }
    spec is_on {
        ensures result == (t is On);

        proof {
            split on t;
        }
    }
}
