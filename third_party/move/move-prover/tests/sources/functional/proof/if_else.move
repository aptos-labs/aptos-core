// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// flag: --language-version=2.4

// E2E tests for `if/else` in proof blocks, including case splits
// with lemma apply.
module 0x42::proof_if_else {

    // ==================================================================
    // Lemmas used in conditional proofs below.

    spec module {
        lemma mul_comm(a: u64, b: u64) {
            ensures a * b == b * a;
        } proof {
            assume [trusted] true;
        }

        lemma div_self(a: u64) {
            requires a > 0;
            ensures a / a == 1;
        } proof {
            assume [trusted] true;
        }
    }

    // ==================================================================
    // Basic case split: max function.

    fun max(a: u64, b: u64): u64 {
        if (a >= b) { a } else { b }
    }
    spec max {
        ensures result >= a;
        ensures result >= b;
        ensures result == a || result == b;
    } proof {
        if (a >= b) {
            post assert result == a;
            assert a >= a;
            assert a >= b;
        } else {
            post assert result == b;
            assert b > a;
            assert b >= b;
        }
    }

    // ==================================================================
    // Three-way classification using nested if/else.

    fun classify(score: u64): u64 {
        if (score < 50) { 0 }
        else if (score < 90) { 1 }
        else { 2 }
    }
    spec classify {
        ensures result <= 2;
        ensures score < 50 ==> result == 0;
        ensures (score >= 50 && score < 90) ==> result == 1;
        ensures score >= 90 ==> result == 2;
    } proof {
        if (score < 50) {
            post assert result == 0;
        } else if (score < 90) {
            post assert result == 1;
            assert score >= 50;
        } else {
            post assert result == 2;
            assert score >= 90;
        }
    }

    // ==================================================================
    // Clamp with let bindings in branches.

    fun clamp(x: u64, lo: u64, hi: u64): u64 {
        if (x < lo) { lo }
        else if (x > hi) { hi }
        else { x }
    }
    spec clamp {
        requires lo <= hi;
        ensures result >= lo;
        ensures result <= hi;
    } proof {
        if (x < lo) {
            let r = lo;
            assert r >= lo;
            assert r <= hi;
        } else if (x > hi) {
            let r = hi;
            assert r >= lo;
            assert r <= hi;
        } else {
            assert x >= lo;
            assert x <= hi;
        }
    }

    // ==================================================================
    // Case split with lemma apply in one branch.

    fun normalize_or_zero(x: u64, y: u64): u64 {
        if (y == 0) { 0 } else { x / y * y }
    }
    spec normalize_or_zero {
        ensures result <= x;
    } proof {
        if (y == 0) {
            post assert result == 0;
            assert 0 <= x;
        } else {
            apply div_self(y);
            assert x / y * y <= x;
        }
    }

    // ==================================================================
    // Three-way split with lemma apply in one branch.

    fun symmetric_op(a: u64, b: u64): u64 {
        if (a == b) { a * 2 }
        else if (a < b) { b - a }
        else { a - b }
    }
    spec symmetric_op {
        requires a * 2 <= MAX_U64;
        ensures a != b ==> result == if (a < b) { b - a } else { a - b };
        ensures a == b ==> result == 2 * a;
    } proof {
        if (a == b) {
            apply mul_comm(a, 2);
            post assert result == 2 * a;
        } else if (a < b) {
            let diff = b - a;
            post assert diff == result;
        } else {
            let diff = a - b;
            post assert diff == result;
        }
    }

    // ==================================================================
    // FAILURE: Wrong assertion in else branch.
    // The else branch asserts result == a, but result is actually b.

    fun min(a: u64, b: u64): u64 {
        if (a <= b) { a } else { b }
    }
    spec min {
        ensures result <= a;
        ensures result <= b;
    } proof {
        if (a <= b) {
            post assert result == a;
        } else {
            post assert result == a;  // error: result == b, not a
        }
    }
}
