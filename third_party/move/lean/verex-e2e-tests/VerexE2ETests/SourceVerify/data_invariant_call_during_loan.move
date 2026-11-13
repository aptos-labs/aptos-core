// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A call made while a loan of a resource is live. The borrow writes a
// prophecy into the slot, so the memory at the call satisfies no invariant
// of that resource until the loan dies; a callee that reaches only another
// resource must not make the caller state one of it.
module 0x42::data_invariant_call_during_loan {
    struct Held has key { count: u64 }
    spec Held {
        invariant count >= 10;
    }

    struct Other has key { n: u64 }
    spec Other {
        invariant n >= 10;
    }

    fun read_other(addr: address): u64 {
        Other[addr].n
    }
    spec read_other {
        pragma opaque;
        pragma aborts_if_is_partial;
        ensures result >= 10;
    }

    public fun call_during_loan(addr: address): u64 {
        let held = &mut Held[addr];
        let read = read_other(addr);
        held.count = held.count + read;
        read
    }
    spec call_during_loan {
        pragma aborts_if_is_partial;
        modifies global<Held>(addr);
        ensures result >= 10;
    }

    // The same call with no loan live across it: the control.
    public fun call_before_loan(addr: address): u64 {
        let read = read_other(addr);
        let held = &mut Held[addr];
        held.count = held.count + read;
        read
    }
    spec call_before_loan {
        pragma aborts_if_is_partial;
        modifies global<Held>(addr);
        ensures result >= 10;
    }
}
