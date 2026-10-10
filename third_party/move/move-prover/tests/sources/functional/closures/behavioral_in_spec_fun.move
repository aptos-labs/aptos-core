// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A behavioral predicate inside a spec function.
//
// The predicate's evaluator is defined over the target function's memory, so
// the spec function must carry that memory -- and the target's pre-state
// memory -- as parameters, exactly as a function specification using the same
// predicate does. Without that, the spec function's Boogie translation names
// `Counter`'s memory as a global and wraps its pre-state in `old(..)`, and
// Boogie rejects both inside a function.
module 0x42::behavioral_in_spec_fun {
    struct Counter has key {
        value: u64,
    }

    fun bump(addr: address) acquires Counter {
        let counter = &mut Counter[addr];
        counter.value = counter.value + 1;
    }
    spec bump {
        aborts_if !exists<Counter>(addr);
        aborts_if Counter[addr].value + 1 > MAX_U64;
        ensures Counter[addr].value == old(Counter[addr].value) + 1;
    }

    // Neither `Counter` nor `old` is mentioned here; both come from `bump`'s
    // contract through the predicate.
    spec fun bump_aborts(addr: address): bool {
        aborts_of<bump>(addr)
    }

    // Two-state: `ensures_of` observes `bump`'s old state, so this spec fun
    // takes `Counter`'s memory in both states.
    spec fun bumped(addr: address): bool {
        ensures_of<bump>(addr)
    }

    fun bump_once(addr: address) acquires Counter {
        bump(addr);
    }
    spec bump_once {
        aborts_if bump_aborts(addr);
        ensures bumped(addr);
    }

    fun bump_twice(addr: address) acquires Counter {
        bump(addr);
        bump(addr);
    }
    spec bump_twice {
        aborts_if bump_aborts(addr);
        aborts_if Counter[addr].value + 2 > MAX_U64;
        ensures Counter[addr].value == old(Counter[addr].value) + 2;
    }
}
