// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Error tests for proof block constructs.
module 0x42::TestProofErrors {

    // ==========================================================================
    // Error: assume without [trusted] annotation.

    fun test_assume_no_trusted(x: u64): u64 {
        x + 1
    }
    spec test_assume_no_trusted {
    } proof {
        assume x > 0;
    }

    // ==========================================================================
    // Error: nested `post post` is not allowed.

    fun test_nested_post(x: u64): u64 { x }
    spec test_nested_post {
        ensures result == x;
    } proof {
        post post assert result == x;
    }

    // ==========================================================================
    // Error: `post` is not allowed in lemma proofs.

    spec module {
        lemma add_zero(a: u64) {
            ensures a + 0 == a;
        } proof {
            post assert a + 0 == a;
        }
    }

    // ==========================================================================
    // Error: proof block on non-function spec.

    spec module {
    } proof {
        assert true;
    }
}
