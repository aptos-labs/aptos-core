// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Tests that behavioral predicates (result_of<f>, aborts_of<f>) work correctly
// when `f` is an opaque function.

module 0x1::opaque_behavioral {

    // ---- Opaque function ----

    fun add_one(x: u64): u64 {
        x + 1
    }
    spec add_one {
        pragma opaque;
        ensures result == x + 1;
        aborts_if x == MAX_U64;
    }

    // ---- Caller using behavioral predicates on opaque function ----

    fun call_add_one(x: u64): u64 {
        add_one(x)
    }
    spec call_add_one {
        ensures result == result_of<add_one>(x);
        aborts_if aborts_of<add_one>(x);
    }

    // ---- Recursive opaque function ----

    spec fun spec_factorial(n: u64): u64 {
        if (n == 0) { 1 } else { n * spec_factorial(n - 1) }
    }

    fun factorial(n: u64): u64 {
        if (n == 0) { 1 } else { n * factorial(n - 1) }
    }
    spec factorial {
        pragma opaque;
        requires n <= 5;
        aborts_if false;
        ensures result == spec_factorial(n);
    }

    // ---- Caller using behavioral predicates on recursive opaque ----

    fun call_factorial(n: u64): u64 {
        factorial(n)
    }
    spec call_factorial {
        requires n <= 5;
        ensures result == result_of<factorial>(n);
        aborts_if aborts_of<factorial>(n);
    }

    // ---- Mutually recursive opaque functions ----

    fun is_even(n: u64): bool {
        if (n == 0) { true } else { is_odd(n - 1) }
    }
    spec is_even {
        pragma opaque;
        aborts_if false;
        ensures result == (n % 2 == 0);
    }

    fun is_odd(n: u64): bool {
        if (n == 0) { false } else { is_even(n - 1) }
    }
    spec is_odd {
        pragma opaque;
        aborts_if false;
        ensures result == (n % 2 == 1);
    }

    fun call_parity(n: u64): bool {
        is_even(n)
    }
    spec call_parity {
        ensures result == result_of<is_even>(n);
        aborts_if aborts_of<is_even>(n);
    }
}
