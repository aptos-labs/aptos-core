// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// A function value invoked in a loop.
module bench::closure_loop {
    fun add(x: u64, y: u64): u64 {
        x + y
    }
    spec add {
        aborts_if x + y > MAX_U64;
        ensures result == x + y;
    }

    public fun count_up(x: u64, n: u64): u64 {
        let f: |u64| u64 has copy + drop = |z| add(z, 1);
        let i = 0;
        let acc = x;
        while (i < n) {
            acc = f(acc);
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant acc == x + i;
        };
        acc
    }
    spec count_up {
        requires x + n <= MAX_U64;
        aborts_if false;
        ensures result == x + n;
    }
}
