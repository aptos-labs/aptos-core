// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Locals of one name in nested scopes are distinct: the bounds of nested
// `for` loops, and a binding that shadows an outer one.
module 0x42::scopes {
    public fun sum_grid(n: u64, m: u64): u64 {
        let sum = 0;
        for (i in 0..n) {
            for (j in 0..m) {
                sum = sum + 1;
            } spec {
                invariant j <= m;
                invariant sum == i * m + j;
            };
        } spec {
            invariant i <= n;
            invariant sum == i * m;
        };
        sum
    }
    spec sum_grid {
        requires n < 1000 && m < 1000;
        aborts_if false;
        ensures result == n * m;
    }

    public fun shadowed(n: u64): u64 {
        let x = n;
        let y = {
            let x = 7;
            x + 1
        };
        x + y
    }
    spec shadowed {
        requires n < 1000;
        ensures result == n + 8;
    }
}
