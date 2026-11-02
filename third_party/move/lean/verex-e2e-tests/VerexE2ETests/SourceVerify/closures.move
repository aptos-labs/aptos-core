// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// Function values verified from their Move specifications: a lambda over a
/// specified function invokes through that function's specification.
module 0x42::closures {
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

    fun sub(x: u64, y: u64): u64 {
        x - y
    }
    spec sub {
        aborts_if x < y;
        ensures result == x - y;
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
        let f = |z| sub(z, y);
        f(x)
    }
    spec trailing {
        aborts_if x < y;
        ensures result == x - y;
    }

    public fun field(x: u64, y: u64): u64 {
        let op = Op { f: |z| add(x, z) };
        (op.f)(y)
    }
    spec field {
        aborts_if x + y > MAX_U64;
        ensures result == x + y;
    }
}
