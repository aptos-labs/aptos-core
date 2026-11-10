// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A mutable reference joined from borrows of different owners must write back
// only to the owner it actually came from. Resolving the join against every
// dying loan's own lender wrote one value into all of their owners and
// constrained owners the reference never came from, which made the state
// contradictory, so false postconditions became provable.

module 0x42::joined_mut_ref_lenders {

    struct A has key { x: u64 }
    struct B has key { x: u64 }

    struct T has drop { x: u64 }
    spec T {
        invariant x > 1;
    }

    /// Writes zero through one resource and reads the field of the other, so
    /// the value returned was never written.
    public fun globals_other(choose_a: bool, addr: address): u64 {
        let a = &mut A[addr].x;
        let b = &mut B[addr].x;
        let selected = if (choose_a) a else b;
        *selected = 0;
        if (choose_a) B[addr].x else A[addr].x
    }

    spec globals_other {
        requires exists<A>(addr) && exists<B>(addr);
        ensures result == 0; // error: the field returned was never written
    }

    /// Must verify: reads back the field that was written through the join.
    public fun globals_same(choose_a: bool, addr: address): u64 {
        let a = &mut A[addr].x;
        let b = &mut B[addr].x;
        let selected = if (choose_a) a else b;
        *selected = 0;
        if (choose_a) A[addr].x else B[addr].x
    }

    spec globals_same {
        requires exists<A>(addr) && exists<B>(addr);
        ensures result == 0;
    }

    /// Must verify: a join of borrows of two local roots writes back to the
    /// one it came from, and to that one only.
    fun locals_written_once(cond: bool): u64 {
        let p = T { x: 3 };
        let q = T { x: 4 };
        let s = if (cond) { &mut p.x } else { &mut q.x };
        *s = 2;
        p.x + q.x
    }

    spec locals_written_once {
        ensures result == (if (cond) 6 else 5);
    }

    /// Rejected: a join that breaks the data invariant of the owner it
    /// reaches owes it where the loan dies.
    fun locals_broken(cond: bool): u64 {
        let p = T { x: 3 };
        let q = T { x: 4 };
        let s = if (cond) { &mut p.x } else { &mut q.x };
        *s = 0;
        p.x + q.x
    }

    // The independent review's reproducer: a join of a borrow of global memory
    // and a reborrow of a parameter's field, both of the same referent type.
    // The holder may hold either, so both lenders' obligations are owed; with
    // only one of them the state became contradictory and `ensures false` was
    // provable by a short authored proof. Both branch orders are kept, because
    // the defect made the verdict depend on the order.

    struct G has key, store { v: u64 }
    spec G {
        invariant v == 0;
    }

    struct Holder { inner: G }

    /// Rejected: the write breaks `G`'s invariant whichever branch it reaches.
    public fun mixed_global_broken(b: bool, addr: address, o: &mut Holder) acquires G {
        let r = if (b) &mut G[addr] else &mut o.inner;
        r.v = 1;
    }

    /// Rejected for the same reason, with the branches swapped.
    public fun mixed_global_broken_swapped(b: bool, addr: address, o: &mut Holder) acquires G {
        let r = if (b) &mut o.inner else &mut G[addr];
        r.v = 1;
    }

    /// Must verify: the same join, writing a value the invariant allows.
    public fun mixed_global_ok(b: bool, addr: address, o: &mut Holder) acquires G {
        let r = if (b) &mut G[addr] else &mut o.inner;
        r.v = 0;
    }

    /// Must verify: the same, branches swapped.
    public fun mixed_global_ok_swapped(b: bool, addr: address, o: &mut Holder) acquires G {
        let r = if (b) &mut o.inner else &mut G[addr];
        r.v = 0;
    }
}
