// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

module 0x42::no_abort {
    spec module {
        pragma aborts_if_is_strict;
    }

    // A failing path of a strict function without `aborts_if` owes `no abort`,
    // at the `spec` item, with its site.
    fun divide(a: u64, b: u64): u64 {
        a / b // error: a division by zero
    }
    spec divide {
        ensures true;
    }

    fun checked(x: u64): u64 {
        assert!(x > 0, 1); // error: the assertion
        x
    }
    spec checked {
        ensures result == x;
    }

    fun element(v: &vector<u64>, i: u64): u64 {
        v[i] // error: one operation, its check and its read
    }
    spec element {
        ensures true;
    }

    fun calling(x: u64): u64 {
        checked(x) // error: a call
    }
    spec calling {
        ensures result == x;
    }

    fun guarded(a: u64, b: u64): u64 {
        if (b == 0) 0 else a / b
    }
    spec guarded {
        ensures b == 0 ==> result == 0;
    }

    // A clause permitting the abort leaves `no abort` out, and keeps its number.
    fun permitted(a: u64, b: u64): u64 {
        a / b
    }
    spec permitted {
        aborts_if b == 0;
    }

    // A caller reasons over a strict callee's contract: it does not abort.
    fun opaque_identity(x: u64): u64 {
        x
    }
    spec opaque_identity {
        pragma opaque;
        ensures result == x;
    }

    fun through_contract(x: u64): u64 {
        opaque_identity(x)
    }
    spec through_contract {
        ensures result == x;
    }

    // One of two sites by its selector, the other by the default.
    fun routed(a: u64, b: u64): u64 {
        a + 1 + b
    }
    spec routed {
        requires a <= 10;
        requires b <= 10;
    }
}
