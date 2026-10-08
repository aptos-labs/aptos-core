// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A loop invariant that no loop header begins with belongs to no loop. As
// in the Move Prover, each is an error and nothing is verified.
module 0x42::loop_invariant_placement_false {
    fun in_body(n: u64): u64 {
        let i = 0;
        while (i < n) {
            spec {
                invariant i < n; // error: in the body
            };
            i = i + 1;
        };
        i
    }

    fun after_assert(n: u64): u64 {
        let i = 0;
        while ({
            spec {
                invariant i <= n;
                assert i <= n;
                invariant i >= 0; // error: after an assertion
            };
            i < n
        }) {
            i = i + 1;
        };
        i
    }

    fun after_statement(n: u64): u64 {
        let i = 0;
        loop {
            if (i >= n) break;
            spec {
                invariant i < n; // error: after a statement
            };
            i = i + 1;
        };
        i
    }
}
