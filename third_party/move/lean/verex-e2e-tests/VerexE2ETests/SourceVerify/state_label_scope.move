// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A state label binds no value, and a quantifier's range reads the contract's
// bindings before it.
module 0x42::state_label_scope {
    spec fun lt_100(x: &mut u64): bool {
        x < 100
    }

    fun keep(x: &mut u64) {
        *x = *x;
    }

    fun label_keeps_the_binding(x: &mut u64) {
        keep(x)
    }
    spec label_keeps_the_binding {
        let n = 1;
        ensures forall n in *, j in 0..n: j == 0 || (n.. |~ lt_100(x));
    }

    fun range_over_binding(): u64 { 0 }
    spec range_over_binding {
        let n = 3;
        ensures forall j in 0..n: j < 3;
    }

    fun range_over_binding_wrong(): u64 { 0 }
    spec range_over_binding_wrong {
        let n = 3;
        ensures forall j in 0..n: j < 2; // error: the range holds 2
    }
}
