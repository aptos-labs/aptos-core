// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// flag: --language-version=2.4

// E2E tests for `post` prefix in proof blocks.
module 0x42::proof_post {

    // ==================================================================
    // Basic post assert with result.

    fun double(x: u64): u64 {
        x + x
    }
    spec double {
        requires x + x <= MAX_U64;
        ensures result == 2 * x;
    } proof {
        // entry-point assertion (no result)
        assert x + x == 2 * x;
        // return-point assertion (uses result)
        post assert result == x + x;
    }

    // ==================================================================
    // Mixed entry and post statements.

    fun add_one(x: u64): u64 {
        x + 1
    }
    spec add_one {
        requires x < MAX_U64;
        ensures result > x;
    } proof {
        assert x + 1 > x;
        post assert result == x + 1;
    }

    // ==================================================================
    // Post with let bindings that precede it.

    fun scale(x: u64, factor: u64): u64 {
        x * factor
    }
    spec scale {
        requires x * factor <= MAX_U64;
        ensures result == x * factor;
    } proof {
        let expected = x * factor;
        post assert result == expected;
    }

    // ==================================================================
    // Post inside if/else branches.

    fun max(a: u64, b: u64): u64 {
        if (a >= b) { a } else { b }
    }
    spec max {
        ensures result >= a;
        ensures result >= b;
    } proof {
        if (a >= b) {
            post assert result == a;
        } else {
            post assert result == b;
        }
    }

    // ==================================================================
    // Let inside a post block: the let is a post-state let, and the
    // assertion that uses it is emitted at return points.

    fun triple(x: u64): u64 {
        x * 3
    }
    spec triple {
        requires x * 3 <= MAX_U64;
        ensures result == x * 3;
    } proof {
        post {
            let expected = x * 3;
            assert expected == result;
        }
    }

    // ==================================================================
    // Pre-state let combined with post block containing its own let.

    fun shift_add(x: u64, y: u64): u64 {
        x * 2 + y
    }
    spec shift_add {
        requires x * 2 + y <= MAX_U64;
        ensures result == x * 2 + y;
    } proof {
        let doubled = x * 2;
        assert doubled + y <= MAX_U64;
        post {
            let expected = doubled + y;
            assert result == expected;
        }
    }

    // ==================================================================
    // FAILURE: False post assertion.
    // result == x but the proof asserts result > x.

    fun identity(x: u64): u64 {
        x
    }
    spec identity {
        ensures result == x;
    } proof {
        post assert result > x;  // error: result == x, not > x
    }
}
