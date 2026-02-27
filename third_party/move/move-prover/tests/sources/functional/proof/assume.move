// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// flag: --language-version=2.4

// E2E tests for `assume [trusted]` in proof blocks.
module 0x42::proof_assume {

    // ==================================================================
    // Trusted assumption enabling overflow-safe multiplication.
    // Without the trusted assumption, the prover cannot verify that
    // x * x does not overflow, since u64 multiplication can wrap.

    fun square(x: u64): u64 {
        x * x
    }
    spec square {
        requires x <= 4294967295; // sqrt(MAX_U64) ~ 2^32
        ensures result == x * x;
    } proof {
        assume [trusted] x * x <= MAX_U64;
    }

    // ==================================================================
    // Trusted assumption chained with assertions that build on it.
    // We trust that a and b are coprime-like (their product is bounded),
    // then use that fact to prove a chain of properties about the result.

    fun product_plus_sum(a: u64, b: u64): u64 {
        a * b + a + b
    }
    spec product_plus_sum {
        requires a <= 1000;
        requires b <= 1000;
        ensures result == a * b + a + b;
        ensures result >= a + b;
    } proof {
        assume [trusted] a * b <= MAX_U64 - a - b;
        assert a * b + a + b >= a + b;
    }

    // ==================================================================
    // Trusted assumption used to establish a bound that enables
    // division reasoning. We trust a non-trivial arithmetic fact
    // about integer division rounding.

    fun avg(a: u64, b: u64): u64 {
        a / 2 + b / 2 + (a % 2 + b % 2) / 2
    }
    spec avg {
        ensures result <= (a + b) / 2;
    } proof {
        // The sum-of-halves formula rounds down, so it never exceeds the true average.
        assume [trusted] a / 2 + b / 2 + (a % 2 + b % 2) / 2 <= (a + b) / 2;
    }
}
