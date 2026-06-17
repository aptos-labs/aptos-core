// separate_baseline: path
// Copyright © Aptos Foundation
// Update invariants across conditional global borrows: the `old(..)` snapshot of a
// resolved borrow's state transition is taken at its borrow site, on every path —
// a reference dying on two branches, a reference borrowed from either of two
// resources, and a ghost-variable update all check against their own pre-state.
module 0x42::update_inv_cond_borrow {
    struct T has key { v: u64 }

    spec module {
        invariant update forall a: address where old(exists<T>(a)):
            global<T>(a).v >= old(global<T>(a).v);
    }

    fun bump_and_die_on_branches(a: address, c: bool) acquires T {
        let r = &mut T[a];
        if (c) {
            r.v = r.v + 1;
        } else {
            r.v = r.v + 2;
        }
    }

    fun bump_one_of(a: address, b: address, c: bool) acquires T {
        let r = if (c) &mut T[a] else &mut T[b];
        r.v = r.v + 1;
    }

    fun clear_one_of(a: address, b: address, c: bool) acquires T {
        let r = if (c) &mut T[a] else &mut T[b];
        r.v = 0; // error: global memory invariant does not hold
    }

    spec module {
        global total: num;
        invariant update total == old(total) + 1;
    }

    fun tick() {
    }
    spec tick {
        update total = total + 1;
    }
}
