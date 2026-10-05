// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// `&mut` references into different resource types at the same address do not alias.

module 0x42::borrow_global_distinct_types {

    struct A has key {
        x: u64
    }

    struct B has key {
        x: u64
    }

    /// Writes zero through one resource and reads the field of the other, so the
    /// value returned was never written. With `A.x = 1` and `B.x = 2` this returns 2
    /// when `choose_a` holds and 1 otherwise, so it is never 0.
    public fun write_zero_and_return_other(choose_a: bool, addr: address): u64 {
        let a = &mut A[addr].x;
        let b = &mut B[addr].x;
        let selected = if (choose_a) a else b;
        *selected = 0;
        if (choose_a) B[addr].x else A[addr].x
    }

    spec write_zero_and_return_other {
        requires exists<A>(addr) && exists<B>(addr);
        ensures result == 0; // error: the field returned was never written
    }

    /// Must verify: reads back the field that was written.
    public fun write_zero_and_return_same(choose_a: bool, addr: address): u64 {
        let a = &mut A[addr].x;
        let b = &mut B[addr].x;
        let selected = if (choose_a) a else b;
        *selected = 0;
        if (choose_a) A[addr].x else B[addr].x
    }

    spec write_zero_and_return_same {
        pragma heartbeats = 50;
        requires exists<A>(addr) && exists<B>(addr);
        ensures result == 0;
    }
}
