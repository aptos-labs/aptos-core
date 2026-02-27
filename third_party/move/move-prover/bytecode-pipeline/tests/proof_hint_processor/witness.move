// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Tests for the `witness` proof hint showing existential witness substitution
// in the bytecode pipeline output.
module 0x42::TestWitness {

    // ==========================================================================
    // Simple witness: provide a constant witness

    fun test_witness_constant(): u64 {
        42
    }
    spec test_witness_constant {
        ensures exists x: u64: x == 42;

        proof {
            witness x = 42 in exists x: u64: x == 42;
        }
    }

    // ==========================================================================
    // Witness using function parameter

    spec fun is_positive(x: u64): bool { x > 0 }

    fun test_witness_param(x: u64): u64 {
        x + 1
    }
    spec test_witness_param {
        ensures exists y: u64: is_positive(y) && y == result;

        proof {
            witness y = x + 1 in exists y: u64: is_positive(y) && y == result;
        }
    }

    // ==========================================================================
    // Witness with where clause (condition)

    fun test_witness_condition(x: u64): u64 {
        x
    }
    spec test_witness_condition {
        requires x > 10;
        ensures exists y: u64 where y > 5: y == x;

        proof {
            witness y = x in exists y: u64 where y > 5: y == x;
        }
    }
}
