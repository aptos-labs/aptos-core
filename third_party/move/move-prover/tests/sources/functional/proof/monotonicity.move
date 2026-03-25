// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// flag: --language-version=2.4

// E2E tests for proofs using lemmas for monotonicity and transitivity.
// The lemmas here have no proof body, so the prover must discharge them as VCs.
module 0x42::proof_monotonicity {

    // ==================================================================
    // Lemma declarations — no proof body means the prover verifies them.

    spec module {
        // Addition is monotone: if a <= b, then a + c <= b + c.
        lemma add_mono(a: u64, b: u64, c: u64) {
            requires a <= b;
            requires b + c <= MAX_U64;
            ensures a + c <= b + c;
        }

        // Commutativity of addition.
        lemma add_comm(a: u64, b: u64) {
            ensures a + b == b + a;
        }
    }

    // ==================================================================
    // Apply monotonicity lemma to prove a bound on a shifted value.
    // If x <= y, then x + offset <= y + offset.

    fun add_offset(x: u64, y: u64, offset: u64): (u64, u64) {
        (x + offset, y + offset)
    }
    spec add_offset {
        requires x <= y;
        requires y + offset <= MAX_U64;
        ensures result_1 <= result_2;
    } proof {
        apply add_mono(x, y, offset);
    }

    // ==================================================================
    // Chain monotonicity with commutativity to prove a reordered bound.
    // Show that offset + x <= offset + y by rewriting both sides.

    fun add_offset_reversed(x: u64, y: u64, offset: u64): (u64, u64) {
        (offset + x, offset + y)
    }
    spec add_offset_reversed {
        requires x <= y;
        requires y + offset <= MAX_U64;
        ensures result_1 <= result_2;
    } proof {
        apply add_mono(x, y, offset);
        apply add_comm(x, offset);
        apply add_comm(y, offset);
    }

    // ==================================================================
    // Multiple lemma applies in sequence building up a conclusion.
    // Prove a + 1 + 1 == a + 2 step by step.

    fun double_increment(a: u64): u64 {
        a + 1 + 1
    }
    spec double_increment {
        requires a + 2 <= MAX_U64;
        ensures result == a + 2;
    } proof {
        let step1 = a + 1;
        let step2 = step1 + 1;
        assert step2 == a + 2;
    }

    // ==================================================================
    // FAILURE: Lemma requires not satisfied at apply site.
    // Missing x <= y precondition for add_mono.

    fun add_unchecked(x: u64, y: u64, offset: u64): (u64, u64) {
        (x + offset, y + offset)
    }
    spec add_unchecked {
        // Missing: requires x <= y
        requires x + offset <= MAX_U64;
        requires y + offset <= MAX_U64;
        ensures result_1 <= result_2;
    } proof {
        apply add_mono(x, y, offset);  // error: x <= y not guaranteed
    }
}
