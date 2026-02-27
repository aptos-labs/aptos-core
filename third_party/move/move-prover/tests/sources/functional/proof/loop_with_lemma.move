// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// flag: --language-version=2.4

// Test: entry-point forall-apply makes lemma available for loop VCs.
module 0x42::proof_loop_with_lemma {

    spec module {
        lemma add_mono(a: u64, b: u64, c: u64) {
            requires a <= b;
            requires a + c <= MAX_U64;
            requires b + c <= MAX_U64;
            ensures a + c <= b + c;
        } proof {
            assume [trusted] true;
        }
    }

    // Count from 0 to n.
    fun count_to(n: u64): u64 {
        let i = 0;
        while (i < n) {
            i = i + 1;
        } spec {
            invariant i <= n;
        };
        i
    }
    spec count_to {
        ensures result == n;
    } proof {
        // Entry-point lemma instantiation — available during loop VC.
        forall a: u64, b: u64, c: u64 apply add_mono(a, b, c);
        post assert result == n;
    }

    // ==================================================================
    // FAILURE: Wrong post assertion after loop.
    // The function returns n, but the proof claims result > n.

    fun count_up(n: u64): u64 {
        let i = 0;
        while (i < n) {
            i = i + 1;
        } spec {
            invariant i <= n;
        };
        i
    }
    spec count_up {
        ensures result == n;
    } proof {
        post assert result > n;  // error: result == n, not > n
    }
}
