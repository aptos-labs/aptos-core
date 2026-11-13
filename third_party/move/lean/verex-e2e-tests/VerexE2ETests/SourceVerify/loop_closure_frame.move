// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A loop whose invariant names a function value keeps the store: the
// invariant is not evaluated, and the function value is only built there.
module 0x42::loop_closure_frame {
    struct Counter has key { value: u64 }

    fun double(x: u64): u64 {
        x * 2
    }
    spec double {
        requires x < 1000;
        aborts_if false;
        ensures result == x * 2;
    }

    fun doubled_up_to(n: u64): u64 {
        let i = 0;
        let last = 0;
        while (i < n) {
            last = double(i);
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant i > n ==> ensures_of<double>(i, last);
        };
        last
    }
    spec doubled_up_to {
        requires n < 1000;
        ensures exists<Counter>(@0x1) == old(exists<Counter>(@0x1));
    }

    fun doubled_wrong(n: u64): u64 {
        let i = 0;
        let last = 0;
        while (i < n) {
            last = double(i);
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant i > n ==> ensures_of<double>(i, last);
        };
        last
    }
    spec doubled_wrong {
        requires n < 1000;
        ensures result == n * 2; // error: the last value doubles n - 1
    }
}
