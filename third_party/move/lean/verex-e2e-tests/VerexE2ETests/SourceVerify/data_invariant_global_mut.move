// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A mutation of a stored resource owes its data invariant where the borrow's
// root is written back, as the Move Prover owes `PackRefDeep` at a write-back
// to a global root. A narrowed borrow owes the resource's invariant, the root
// of the borrow being the resource, and a function that only mutates such a
// resource is verified for the invariant without a specification of its own.
module 0x42::data_invariant_global_mut {
    struct Tally has key { count: u64 }
    spec Tally {
        invariant count > 0;
    }

    public fun bump(addr: address) {
        let t = &mut Tally[addr];
        t.count = t.count + 1;
    }
    spec bump {
        pragma aborts_if_is_partial;
        modifies global<Tally>(addr);
    }

    public fun zero(addr: address) {
        let t = &mut Tally[addr];
        t.count = 0; // error: the invariant fails where the mutation ends
    }
    spec zero {
        pragma aborts_if_is_partial;
        modifies global<Tally>(addr);
    }

    public fun zero_field(addr: address) {
        let count = &mut Tally[addr].count;
        *count = 0; // error: the invariant fails where the mutation ends
    }
    spec zero_field {
        pragma aborts_if_is_partial;
        modifies global<Tally>(addr);
    }

    public fun zero_spec_less(addr: address) {
        let t = &mut Tally[addr];
        t.count = 0; // error: the invariant fails where the mutation ends
    }
}
