// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// Function values: closures capturing values, stored in structs, and
/// passed to higher-order functions with bodies.
module bench::closures {
    struct Op has copy, drop {
        f: |u64| u64 has copy + drop,
    }

    fun add(x: u64, y: u64): u64 {
        x + y
    }
    spec add {
        aborts_if x + y > MAX_U64;
        ensures result == x + y;
    }

    fun mul(x: u64, y: u64): u64 {
        x * y
    }
    spec mul {
        aborts_if x * y > MAX_U64;
        ensures result == x * y;
    }

    /// Not opaque: a caller sees through the body.
    fun twice(f: |u64| u64 has copy + drop, x: u64): u64 {
        f(f(x))
    }

    public fun leading(x: u64, y: u64): u64 {
        let f = |z| add(x, z);
        f(y)
    }
    spec leading {
        aborts_if x + y > MAX_U64;
        ensures result == x + y;
    }

    public fun trailing(x: u64, y: u64): u64 {
        let f = |z| mul(z, y);
        f(x)
    }
    spec trailing {
        aborts_if x * y > MAX_U64;
        ensures result == x * y;
    }

    public fun two_captures(x: u64, a: u64, b: u64): u64 {
        let f = |z| add(add(z, a), b);
        f(x)
    }
    spec two_captures {
        aborts_if x + a > MAX_U64;
        aborts_if x + a + b > MAX_U64;
        ensures result == x + a + b;
    }

    public fun field(x: u64, y: u64): u64 {
        let op = Op { f: |z| add(x, z) };
        (op.f)(y)
    }
    spec field {
        aborts_if x + y > MAX_U64;
        ensures result == x + y;
    }

    public fun choose(flag: bool, x: u64): u64 {
        let op = if (flag) Op { f: |z| add(z, 1) } else Op { f: |z| mul(z, 2) };
        (op.f)(x)
    }
    spec choose {
        aborts_if flag && x + 1 > MAX_U64;
        aborts_if !flag && x * 2 > MAX_U64;
        ensures flag ==> result == x + 1;
        ensures !flag ==> result == x * 2;
    }

    public fun add_twice(x: u64, k: u64): u64 {
        twice(|z| add(z, k), x)
    }
    spec add_twice {
        aborts_if x + k > MAX_U64;
        aborts_if x + 2 * k > MAX_U64;
        ensures result == x + 2 * k;
    }
}
