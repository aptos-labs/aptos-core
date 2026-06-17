// Copyright © Aptos Foundation
// Unlabeled calls of spec functions whose bodies contain behavioral
// predicates over concrete functions. The predicate target's spec memory is
// folded into the spec function's memory parameters: a one-state predicate
// (`aborts_of`) keeps the spec function single-state, a two-state predicate
// (`ensures_of`) makes it dual-state (pre/post parameter pairs).
module 0x42::unlabeled_spec_fun_with_bp {
    struct Counter has key { value: u64 }

    fun bump(a: address) acquires Counter {
        Counter[a].value = Counter[a].value + 1;
    }
    spec bump {
        aborts_if !exists<Counter>(a);
        aborts_if Counter[a].value + 1 > MAX_U64;
    }

    // One-state: `aborts_of` in a single-state spec fun.
    spec fun wont_abort(a: address): bool {
        !aborts_of<bump>(a)
    }

    fun touch(a: address) acquires Counter {
        bump(a);
    }
    spec touch {
        // Checked in both directions: `touch` aborts exactly when `bump` does.
        aborts_if !wont_abort(a);
        ensures Counter[a].value == old(Counter[a].value) + 1;
    }

    // Two-state: `ensures_of` in a dual-state spec fun.
    fun increment(a: address) acquires Counter {
        Counter[a].value = Counter[a].value + 1;
    }
    spec increment {
        pragma opaque;
        modifies Counter[a];
        ensures Counter[a].value == old(Counter[a].value) + 1;
        aborts_if !exists<Counter>(a);
        aborts_if Counter[a].value + 1 > MAX_U64;
    }

    spec fun incremented(a: address): bool {
        ensures_of<increment>(a)
    }

    fun do_inc(a: address) acquires Counter {
        increment(a);
    }
    spec do_inc {
        pragma aborts_if_is_partial;
        ensures incremented(a);
    }

    fun do_inc_twice(a: address) acquires Counter {
        increment(a);
        increment(a);
    }
    spec do_inc_twice {
        pragma aborts_if_is_partial;
        ensures incremented(a); // error: two increments do not satisfy a single increment's ensures
    }
}
