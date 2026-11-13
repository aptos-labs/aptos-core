// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A Move reference temp rebound each loop iteration: the old holder dies alone
// while the loan continues through the new holder.  The verifying function has
// a false twin and an `ensures false` twin.
module 0x42::loop_rebind {
    fun rebind_step(n: u64, a: u64): u64 {
        let x = a;
        let s = &mut x;
        let i = 0;
        while (i < n) {
            *s = i + 1;
            s = &mut x;
            i = i + 1;
        };
        x
    }
    spec rebind_step {
        pragma unroll = 3;
        requires n <= 3;
        ensures n == 0 ==> result == a;
        ensures n >= 1 ==> result == n;
    }

    fun rebind_step_false(n: u64, a: u64): u64 {
        let x = a;
        let s = &mut x;
        let i = 0;
        while (i < n) {
            *s = i + 1;
            s = &mut x;
            i = i + 1;
        };
        x
    }
    spec rebind_step_false {
        pragma unroll = 3;
        requires n <= 3;
        ensures result == a; // error: wrong when n >= 1
    }

    fun rebind_step_vacuous(n: u64, a: u64): u64 {
        let x = a;
        let s = &mut x;
        let i = 0;
        while (i < n) {
            *s = i + 1;
            s = &mut x;
            i = i + 1;
        };
        x
    }
    spec rebind_step_vacuous {
        pragma unroll = 3;
        requires n <= 3;
        ensures false; // error: not vacuous
    }
}
