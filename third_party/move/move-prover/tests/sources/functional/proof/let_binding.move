// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// flag: --language-version=2.4

// E2E tests for `let` bindings in proof blocks.
module 0x42::proof_let_binding {

    // ==================================================================
    // Let binding naming the result, used in assertion chain.

    fun scale_and_offset(x: u64, factor: u64, offset: u64): u64 {
        x * factor + offset
    }
    spec scale_and_offset {
        requires x * factor + offset <= MAX_U64;
        ensures result == x * factor + offset;
    } proof {
        let scaled = x * factor;
        let total = scaled + offset;
        post assert total == result;
    }

    // ==================================================================
    // Multiple sequential lets building up a polynomial computation.
    // Computes x^2 + 2*x + 1 == (x + 1)^2.

    fun square_plus_one(x: u64): u64 {
        (x + 1) * (x + 1)
    }
    spec square_plus_one {
        requires x + 1 <= 4294967295;
        ensures result == (x + 1) * (x + 1);
    } proof {
        let y = x + 1;
        let r = y * y;
        assert r == (x + 1) * (x + 1);
        post assert r == result;
    }

    // ==================================================================
    // Let bindings inside if/else branches, each naming branch-local
    // intermediate values that feed into the final assertion.

    fun abs_diff(a: u64, b: u64): u64 {
        if (a >= b) { a - b } else { b - a }
    }
    spec abs_diff {
        ensures a >= b ==> result == a - b;
        ensures b > a ==> result == b - a;
    } proof {
        if (a >= b) {
            let diff = a - b;
            post assert diff == result;
        } else {
            let diff = b - a;
            post assert diff == result;
        }
    }

    // ==================================================================
    // FAILURE: Wrong let-based assertion (off by one).

    fun offset_by_two(x: u64): u64 {
        x + 2
    }
    spec offset_by_two {
        requires x + 2 <= MAX_U64;
        ensures result == x + 2;
    } proof {
        let expected = x + 3;  // off by one
        post assert expected == result;  // error: x+3 != x+2
    }
}
