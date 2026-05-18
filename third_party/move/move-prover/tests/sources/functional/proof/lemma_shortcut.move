// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// flag: --language-version=2.4

// Tests for `spec lemma` shortcut syntax and `@ident` escape.
module 0x42::proof_lemma_shortcut {

    // ==================================================================
    // `spec lemma` shortcut (no `spec module { ... }` wrapper needed).

    spec lemma add_zero_left(x: u64) {
        ensures 0 + x == x;
    }

    fun identity_add(x: u64): u64 {
        0 + x
    }
    spec identity_add {
        ensures result == x;
    } proof {
        apply add_zero_left(x);
    }

    // ==================================================================
    // Trusted shortcut lemma with preconditions.

    spec lemma mul_comm(a: u64, b: u64) {
        ensures a * b == b * a;
    } proof {
        assume [trusted] true;
    }

    fun mul_swap(a: u64, b: u64): u64 {
        b * a
    }
    spec mul_swap {
        ensures result == a * b;
    } proof {
        apply mul_comm(a, b);
    }

    // ==================================================================
    // A function literally named `lemma`. `spec lemma { ... }` works
    // because `lemma` is followed by `{`, not a name.

    fun lemma(x: u64): u64 {
        x
    }
    spec lemma {
        ensures result == x;
    }

    // ==================================================================
    // FAILURE: Shortcut lemma with false ensures.

    spec lemma false_identity(x: u64) {
        ensures x + 1 == x;  // obviously false
    }
}
