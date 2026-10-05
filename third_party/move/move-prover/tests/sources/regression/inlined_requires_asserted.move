// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A callee's `requires` is asserted at every call, including calls inside a function that
// is inlined into a verified root but not verified itself, and an inlined body assumes no
// `requires` its callers do not check. Each `pub_entry` ensures a false result and must fail.

module 0x42::callee_not_verified {
    fun g(x: u64): u64 {
        x + 1
    }
    spec g {
        pragma verify = false;
        requires x < 10;
        aborts_if false;
        ensures result == x + 1;
    }

    fun f(): u64 {
        g(100)
    }
    spec f {
        pragma verify = false;
    }

    public fun pub_entry(): u64 {
        f()
    }
    spec pub_entry {
        ensures result == 0;
    }
}

module 0x42::callee_verified {
    fun g(x: u64): u64 {
        x + 1
    }
    spec g {
        requires x < 10;
        aborts_if false;
        ensures result == x + 1;
    }

    fun f(): u64 {
        g(100)
    }
    spec f {
        pragma verify = false;
    }

    public fun pub_entry(): u64 {
        f()
    }
    spec pub_entry {
        ensures result == 0;
    }

    public fun direct(): u64 {
        g(100)
    }
    spec direct {
        ensures result == 101;
    }
}

module 0x42::callee_opaque {
    fun g(x: u64): u64 {
        x + 1
    }
    spec g {
        pragma opaque;
        requires x < 10;
        aborts_if false;
        ensures result == x + 1;
    }

    fun f(): u64 {
        g(100)
    }
    spec f {
        pragma verify = false;
    }

    public fun pub_entry(): u64 {
        f()
    }
    spec pub_entry {
        ensures result == 0;
    }
}

module 0x42::requires_established {
    fun g(x: u64): u64 {
        x + 1
    }
    spec g {
        requires x < 10;
        aborts_if false;
        ensures result == x + 1;
    }

    fun f(y: u64): u64 {
        g(y)
    }
    spec f {
        pragma verify = false;
    }

    public fun pub_entry(y: u64): u64 {
        if (y < 5) f(y) else 0
    }
    spec pub_entry {
        ensures y < 5 ==> result == y + 1;
    }
}

module 0x42::callee_concrete_requires {
    fun g(x: u64): u64 {
        x + 1
    }
    spec g {
        requires [concrete] x < 10;
        aborts_if false;
        ensures result == x + 1;
    }

    public fun pub_entry(): u64 {
        g(100)
    }
    spec pub_entry {
        ensures result == 0;
    }
}
