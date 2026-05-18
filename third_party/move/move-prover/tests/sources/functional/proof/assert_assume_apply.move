// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// flag: --language-version=2.4

// E2E tests for assert, assume, and apply in proof blocks.
module 0x42::proof_assert_assume_apply {

    // ============================================================
    // Assert in proof block.

    fun increment(_addr: address): u64 {
        1
    }
    spec increment {
        ensures result == 1;
    } proof {
        assert 1 == 1;
    }

    // ============================================================
    // Assume [trusted] in proof block.

    fun complex_op(x: u64): u64 {
        x * x
    }
    spec complex_op {
        ensures result == x * x;
    } proof {
        assume [trusted] x * x < MAX_U64;
    }

    // ============================================================
    // Apply: instantiate a spec function at specific values.

    spec fun add_commutative(a: u64, b: u64): bool { a + b == b + a }

    fun add_values(a: u64, b: u64): u64 {
        a + b
    }
    spec add_values {
        ensures result == a + b;
    } proof {
        apply add_commutative(a, b);
    }
}
