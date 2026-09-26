// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Two `&mut` references into global storage must not alias when they point into
// different resource types at the same address.
//
// The root of a global borrow is a `$Location`, which used to carry the address and
// nothing else. A field borrow appends only the field offset to the path, so
// `&mut A[addr].x` and `&mut B[addr].x` produced the same location and the same path.
// `$IsSameMutation` and `$IsParentMutation` decide on the location and the path alone,
// so the two counted as one reference: when the write through either was written back,
// the prover updated both memories and the field of the resource that was never
// written also looked changed.
//
// Move keeps resources apart by type as well as by address, so `$Global` now carries
// the identity of the resource type's memory next to the address. Each resource type's
// memory gets a `unique` identity constant, which gives pairwise distinctness across
// all resource types.

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

    /// Reads back the field that really was written. This must verify: separating the
    /// two roots must not cost the legitimate write-back.
    public fun write_zero_and_return_same(choose_a: bool, addr: address): u64 {
        let a = &mut A[addr].x;
        let b = &mut B[addr].x;
        let selected = if (choose_a) a else b;
        *selected = 0;
        if (choose_a) A[addr].x else B[addr].x
    }

    spec write_zero_and_return_same {
        requires exists<A>(addr) && exists<B>(addr);
        ensures result == 0;
    }
}
