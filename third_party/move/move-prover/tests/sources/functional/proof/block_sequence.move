// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// flag: --language-version=2.4

// E2E tests for proof blocks with sequences and nested blocks.
module 0x42::proof_block_sequence {

    // ==================================================================
    // Nested blocks with scoped let bindings that don't leak.
    // Each block introduces a name used only within that block.

    fun sum3(a: u64, b: u64, c: u64): u64 {
        a + b + c
    }
    spec sum3 {
        requires a + b + c <= MAX_U64;
        ensures result == a + b + c;
    } proof {
        {
            let partial = a + b;
            assert partial == a + b;
        }
        {
            let total = a + b + c;
            post assert total == result;
        }
    }

    // ==================================================================
    // Sequential assertions interleaved with lets, building up to
    // a non-trivial conclusion about a multi-step computation.

    fun weighted_avg_x2(x: u64, y: u64): u64 {
        // Computes 2 * ((3*x + y) / 4), an approximate weighted average doubled.
        (3 * x + y) / 4 * 2
    }
    spec weighted_avg_x2 {
        requires 3 * x + y <= MAX_U64;
        ensures result == (3 * x + y) / 4 * 2;
        ensures result <= 3 * x + y;
    } proof {
        let wx = 3 * x;
        let sum = wx + y;
        let half = sum / 4;
        assert half <= sum;
        assert half * 2 <= sum;
    }

    // ==================================================================
    // If/else combined with let sequences: different proof structure
    // per branch, each using lets for clarity.

    fun safe_sub_or_zero(a: u64, b: u64): u64 {
        if (a >= b) { a - b } else { 0 }
    }
    spec safe_sub_or_zero {
        ensures result <= a;
        ensures a >= b ==> result == a - b;
        ensures a < b ==> result == 0;
    } proof {
        if (a >= b) {
            let diff = a - b;
            assert diff <= a;
            post assert diff == result;
        } else {
            let r = 0;
            assert r <= a;
            post assert r == result;
        }
    }

    // ==================================================================
    // FAILURE: False assertion in a block.
    // a + b <= a is only true when b == 0, so this fails in general.

    fun add_pair(a: u64, b: u64): u64 {
        a + b
    }
    spec add_pair {
        requires a + b <= MAX_U64;
        ensures result == a + b;
    } proof {
        {
            assert a + b <= a;  // error: false when b > 0
        }
    }
}
