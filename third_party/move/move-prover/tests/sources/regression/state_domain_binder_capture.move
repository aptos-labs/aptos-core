// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A user binder spelled like the value binder of a state quantifier does not capture it:
// `captured` and `renamed` are alpha-equivalent and get the same verdict.

module 0x42::state_domain_binder_capture {

    spec fun lt_100(x: &mut u64): bool {
        x < 100
    }

    fun keep(x: &mut u64) {
        *x = *x;
    }

    public fun captured(x: &mut u64) {
        keep(x)
    }
    spec captured {
        requires x >= 1000;
        ensures exists s in *: (exists s_val: num: s_val < 100 && (s.. |~ lt_100(x)));
    }

    public fun renamed(x: &mut u64) {
        keep(x)
    }
    spec renamed {
        requires x >= 1000;
        ensures exists s in *: (exists k_val: num: k_val < 100 && (s.. |~ lt_100(x)));
    }
}
