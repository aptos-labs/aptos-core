// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Pipeline tests for assert, assume, and use proof hints.
module 0x42::TestAssertAssumeUse {

    // ==========================================================================
    // Assert and assume hints → Prop bytecode.

    spec fun helper(x: u64): bool { x > 0 }

    fun test_assert_assume(x: u64): u64 {
        x + 1
    }
    spec test_assert_assume {
        ensures result == x + 1;

        proof {
            assert helper(x + 1);
            assume [trusted] x < 18446744073709551615;
        }
    }

    // ==========================================================================
    // Use hint: instantiate a spec function at specific arguments.

    spec fun add_comm(a: u64, b: u64): bool { a + b == b + a }

    fun test_use(a: u64, b: u64): u64 {
        a + b
    }
    spec test_use {
        ensures result == a + b;

        proof {
            use add_comm(a, b);
        }
    }

    // ==========================================================================
    // Generic function with assert hint.

    spec fun is_nonempty<T>(v: vector<T>): bool { len(v) > 0 }

    fun test_generic_assert<T>(v: vector<T>): vector<T> {
        v
    }
    spec test_generic_assert {
        requires len(v) > 0;
        ensures result == v;

        proof {
            assert is_nonempty(v);
        }
    }
}
