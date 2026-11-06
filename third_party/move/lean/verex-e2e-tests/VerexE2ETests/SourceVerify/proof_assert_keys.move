// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// `on assert k` addresses the `k`-th in-body assertion in source order, in a
/// block of one assertion and in a block of several.
module 0x42::proof_assert_keys {
    fun blocks(x: u64): u64 {
        let y = x / 2;
        spec {
            assert x * x >= 0;
        };
        let z = y / 2;
        spec {
            assert (x + 1) * (x + 1) >= 0;
            assert (x + 2) * (x + 2) >= 0;
        };
        z
    }
    spec blocks {
        pragma verify = true;
    }

    // With a specification variable, a closure's run is read by its body.
    spec module {
        global calls: u64;
    }

    fun apply_once<T>(f: |T|T, x: T): T {
        f(x)
    }
    spec apply_once {
        pragma verify = false;
        pragma opaque;
        aborts_if aborts_of<f>(x);
        ensures ensures_of<f>(x, result);
    }

    // The run of a literal closure is read at the block's conjunction: each
    // assertion keeps its key.
    fun after_closure(x: u64): u64 {
        let r = apply_once(|v| v / 2, x);
        spec {
            assert (x + 1) * (x + 1) >= 0;
            assert (x + 2) * (x + 2) >= 0;
        };
        r
    }
    spec after_closure {
        pragma verify = true;
    }
}
