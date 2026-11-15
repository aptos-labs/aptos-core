// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// `apply f.unfold(a)`: a recursive specification function's unfolding at its
// arguments, its guard decided, a fact of the block.
module 0x42::proof_unfold {
    spec fun sum_upto(n: u64): num {
        if (n == 0) 0 else sum_upto(n - 1) + n
    }

    spec fun count<T>(v: vector<T>, n: u64): num {
        if (n == 0) 0 else count(v, n - 1) + 1
    }

    fun step(n: u64) { let _ = n; }
    spec step {
        requires n > 0;
        requires n < 100;
        aborts_if false;
        ensures sum_upto(n) == sum_upto(n - 1) + n;
    }

    fun base() { }
    spec base {
        aborts_if false;
        ensures sum_upto(0) == 0;
    }

    fun generic_step<T: drop>(v: vector<T>, n: u64) { let _ = v; let _ = n; }
    spec generic_step {
        requires n > 0;
        aborts_if false;
        ensures count(v, n) == count(v, n - 1) + 1;
    }
}
