// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// Higher-order functions specified by behavioral predicates, and callers
/// passing lambdas with specifications and named functions.
module bench::behavior {
    public fun apply(f: |u64| u64, x: u64): u64 {
        f(x)
    }
    spec apply {
        pragma opaque;
        aborts_if aborts_of<f>(x);
        ensures ensures_of<f>(x, result);
    }

    public fun apply_result(f: |u64| u64, x: u64): u64 {
        f(x)
    }
    spec apply_result {
        pragma opaque;
        aborts_if aborts_of<f>(x);
        ensures result == result_of<f>(x);
    }

    public fun apply_requires(f: |u64| u64, x: u64): u64 {
        f(x)
    }
    spec apply_requires {
        pragma opaque;
        requires requires_of<f>(x);
        aborts_if aborts_of<f>(x);
        ensures ensures_of<f>(x, result);
    }

    public fun apply2(f: |u64, u64| u64, x: u64, y: u64): u64 {
        f(x, y)
    }
    spec apply2 {
        pragma opaque;
        aborts_if aborts_of<f>(x, y);
        ensures ensures_of<f>(x, y, result);
    }

    public fun apply_twice(f: |u64| u64 has copy, x: u64): u64 {
        f(f(x))
    }
    spec apply_twice {
        pragma opaque;
        aborts_if aborts_of<f>(x) || aborts_of<f>(result_of<f>(x));
        ensures result == result_of<f>(result_of<f>(x));
    }

    fun increment(x: u64): u64 {
        x + 1
    }
    spec increment {
        aborts_if x + 1 > MAX_U64;
        ensures result == x + 1;
    }

    fun halve(x: u64): u64 {
        assert!(x % 2 == 0, 1);
        x / 2
    }
    spec halve {
        aborts_if x % 2 != 0 with 1;
        ensures result == x / 2;
    }

    public fun add_five(x: u64): u64 {
        apply(|y| y + 5 spec {
            aborts_if y + 5 > MAX_U64;
            ensures result == y + 5;
        }, x)
    }
    spec add_five {
        aborts_if x + 5 > MAX_U64;
        ensures result == x + 5;
    }

    public fun checked(x: u64): u64 {
        apply(|y| if (y == 0) abort 1 else y spec {
            aborts_if y == 0;
            ensures result == y;
        }, x)
    }
    spec checked {
        aborts_if x == 0;
        ensures result == x;
    }

    public fun named(x: u64): u64 {
        apply(increment, x)
    }
    spec named {
        aborts_if x + 1 > MAX_U64;
        ensures result == x + 1;
    }

    public fun named_result(x: u64): u64 {
        apply_result(halve, x)
    }
    spec named_result {
        aborts_if x % 2 != 0;
        ensures result == x / 2;
    }

    public fun captured(x: u64, k: u64): u64 {
        apply(|y| y + k spec {
            aborts_if y + k > MAX_U64;
            ensures result == y + k;
        }, x)
    }
    spec captured {
        aborts_if x + k > MAX_U64;
        ensures result == x + k;
    }

    public fun with_requires(x: u64): u64 {
        apply_requires(|y| y * 2, x)
    }
    spec with_requires {
        aborts_if x * 2 > MAX_U64;
        ensures result == x * 2;
    }

    public fun sum(x: u64, y: u64): u64 {
        apply2(|a, b| a + b spec {
            aborts_if a + b > MAX_U64;
            ensures result == a + b;
        }, x, y)
    }
    spec sum {
        aborts_if x + y > MAX_U64;
        ensures result == x + y;
    }

    public fun add_two(x: u64): u64 {
        apply_twice(increment, x)
    }
    spec add_two {
        aborts_if x + 2 > MAX_U64;
        ensures result == x + 2;
    }

    /// The result of a known function at literal arguments.
    public fun known(): u64 {
        increment(41)
    }
    spec known {
        ensures result == result_of<increment>(41);
    }
}
