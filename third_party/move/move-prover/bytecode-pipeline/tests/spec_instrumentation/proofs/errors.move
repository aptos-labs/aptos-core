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

    // ==========================================================================
    // Error: lemma apply with wrong arity.

    spec module {
        lemma arity_lemma(a: u64, b: u64) {
            ensures a + b == b + a;
        }
    }

    fun test_arity_too_few(x: u64): u64 { x }
    spec test_arity_too_few {
        ensures result == x;
    } proof {
        apply arity_lemma(x); // error: expects 2, got 1
    }

    fun test_arity_too_many(x: u64): u64 { x }
    spec test_arity_too_many {
        ensures result == x;
    } proof {
        apply arity_lemma(x, x, x); // error: expects 2, got 3
    }

    // ==========================================================================
    // Error: let binding from then-branch used after if.

    fun test_let_escapes_if(x: u64): u64 { x }
    spec test_let_escapes_if {
        ensures result == x;
    } proof {
        if (x > 0) {
            let y = x + 1;
            assert y > x;
        } else {
            assert true;
        }
        assert y > 0; // error: y is not in scope
    }
}
