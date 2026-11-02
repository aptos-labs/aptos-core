// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A loop takes the leading run of `invariant`s of the specification blocks
// its header begins with, across consecutive blocks, as in the Move Prover.
module 0x42::loop_invariant_placement {
    // The bound comes from the second block of the header.
    fun count(n: u64): u64 {
        let i = 0;
        while ({
            spec {
                invariant i >= 0;
            };
            spec {
                invariant i <= n;
            };
            i < n
        }) {
            i = i + 1;
        };
        i
    }
    spec count {
        ensures result == n;
    }

    // An invariant opening a `loop` body heads every iteration.
    fun count_loop(n: u64): u64 {
        let i = 0;
        loop {
            spec {
                invariant i <= n;
            };
            if (i >= n) break;
            i = i + 1;
        };
        i
    }
    spec count_loop {
        ensures result == n;
    }
}
