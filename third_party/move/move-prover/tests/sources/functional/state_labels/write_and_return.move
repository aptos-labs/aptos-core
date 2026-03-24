// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Tests that behavioral predicates correctly model closures that both write state
// and return post-state-derived values. This exercises:
// - result_of being evaluated against post-havoc memory (not pre-state)
// - ensures_of being assumed after havoc to constrain post-state
// - dual-state memory args (old vs current) for old() in ensures

module 0x42::write_and_return {

    struct Counter has key { value: u64 }

    // =========================================================================
    // Library: opaque functions that write and return
    // =========================================================================

    /// Writes a constant and returns it.
    fun write_and_read(addr: address): u64 acquires Counter {
        Counter[addr].value = 42;
        Counter[addr].value
    }
    spec write_and_read {
        pragma opaque;
        modifies Counter[addr];
        ensures result == 42;
        ensures Counter[addr].value == 42;
    }

    /// Increments Counter and returns the new value.
    fun incr_and_read(addr: address): u64 acquires Counter {
        Counter[addr].value = Counter[addr].value + 1;
        Counter[addr].value
    }
    spec incr_and_read {
        pragma opaque;
        modifies Counter[addr];
        ensures result == old(Counter[addr].value) + 1;
        ensures Counter[addr].value == old(Counter[addr].value) + 1;
        aborts_if !exists<Counter>(addr);
        aborts_if Counter[addr].value + 1 > MAX_U64;
    }

    // =========================================================================
    // Higher-order opaque wrapper that reads+writes
    // =========================================================================

    fun apply_rw(f: |address| u64, x: address): u64 {
        f(x)
    }
    spec apply_rw {
        pragma opaque;
        modifies Counter[x];
        modifies_of<f>(a: address) Counter[a];
        ensures ensures_of<f>(x, result);
        aborts_if aborts_of<f>(x);
    }

    // =========================================================================
    // 1. Write and return constant — positive
    // =========================================================================

    fun test_write_return(addr: address): u64 acquires Counter {
        apply_rw(|a| write_and_read(a) spec {
            modifies Counter[a];
            ensures result == 42;
            ensures Counter[a].value == 42;
        }, addr)
    }
    spec test_write_return {
        pragma aborts_if_is_partial;
        ensures result == 42;
    }

    // =========================================================================
    // 2. Write and return constant — negative (wrong claim)
    // =========================================================================

    fun test_write_return_wrong(addr: address): u64 acquires Counter {
        apply_rw(|a| write_and_read(a) spec {
            modifies Counter[a];
            ensures result == 42;
            ensures Counter[a].value == 42;
        }, addr)
    }
    spec test_write_return_wrong {
        pragma aborts_if_is_partial;
        ensures result == 0; // error: post-condition does not hold
    }

    // =========================================================================
    // 3. Increment and return — exercises old() in ensures (dual-state)
    // =========================================================================

    fun test_incr_return(addr: address): u64 acquires Counter {
        apply_rw(|a| incr_and_read(a) spec {
            modifies Counter[a];
            ensures result == old(Counter[a].value) + 1;
            ensures Counter[a].value == old(Counter[a].value) + 1;
            aborts_if !exists<Counter>(a);
            aborts_if Counter[a].value + 1 > MAX_U64;
        }, addr)
    }
    spec test_incr_return {
        aborts_if !exists<Counter>(addr);
        aborts_if Counter[addr].value + 1 > MAX_U64;
        ensures result == old(Counter[addr].value) + 1;
    }

    // =========================================================================
    // 4. Increment return — negative (wrong old-state claim)
    // =========================================================================

    fun test_incr_return_wrong(addr: address): u64 acquires Counter {
        apply_rw(|a| incr_and_read(a) spec {
            modifies Counter[a];
            ensures result == old(Counter[a].value) + 1;
            ensures Counter[a].value == old(Counter[a].value) + 1;
            aborts_if !exists<Counter>(a);
            aborts_if Counter[a].value + 1 > MAX_U64;
        }, addr)
    }
    spec test_incr_return_wrong {
        aborts_if !exists<Counter>(addr);
        aborts_if Counter[addr].value + 1 > MAX_U64;
        ensures result == old(Counter[addr].value); // error: post-condition does not hold
    }
}
