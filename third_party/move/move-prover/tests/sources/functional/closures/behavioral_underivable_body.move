// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A function without a specification whose body is not exactly describable
// (here: a loop) cannot interpret a behavioral predicate.
module 0x42::behavioral_underivable_body {
    fun sum_to(n: u64): u64 {
        let i = 0;
        let s = 0;
        while (i < n) {
            i = i + 1;
            s = s + i;
        };
        s
    }

    fun call(n: u64): u64 {
        sum_to(n)
    }
    spec call(n: u64): u64 {
        pragma opaque;
        ensures result == result_of<sum_to>(n);
        aborts_if aborts_of<sum_to>(n);
    }
}
