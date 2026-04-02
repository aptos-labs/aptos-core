// Copyright © Aptos Foundation
// Tests for two-state spec functions with state labels and void function
// behavioral predicates.
module 0x42::two_state_labels {
    struct Counter has key { value: u64 }

    // =========================================================================
    // Two-state spec function
    // =========================================================================

    spec fun counter_increased(addr: address): bool {
        old(Counter[addr].value) < Counter[addr].value
    }

    // =========================================================================
    // Opaque increment (void — no return value)
    // =========================================================================

    fun increment(addr: address) acquires Counter {
        Counter[addr].value = Counter[addr].value + 1;
    }
    spec increment {
        pragma opaque;
        modifies Counter[addr];
        ensures counter_increased(addr);
        ensures Counter[addr].value == old(Counter[addr].value) + 1;
        aborts_if !exists<Counter>(addr);
        aborts_if Counter[addr].value + 1 > MAX_U64;
    }

    // =========================================================================
    // 1. Basic: two-state spec fun in ensures (no state labels)
    // =========================================================================

    fun single_increment(addr: address) acquires Counter {
        increment(addr);
    }
    spec single_increment {
        pragma aborts_if_is_partial;
        ensures counter_increased(addr);
    }

    // =========================================================================
    // 2. Void function through behavioral predicate (no state labels)
    // =========================================================================

    fun apply_fn(f: |address|, addr: address) {
        f(addr)
    }
    spec apply_fn {
        pragma opaque;
        modifies Counter[addr];
        modifies_of<f>(a: address) Counter[a];
        ensures ensures_of<f>(addr);
        aborts_if aborts_of<f>(addr);
    }

    fun test_void_bp(addr: address) acquires Counter {
        apply_fn(|a| increment(a) spec {
            modifies Counter[a];
            ensures counter_increased(a);
            ensures Counter[a].value == old(Counter[a].value) + 1;
            aborts_if !exists<Counter>(a);
            aborts_if Counter[a].value + 1 > MAX_U64;
        }, addr);
    }
    spec test_void_bp {
        pragma aborts_if_is_partial;
        ensures Counter[addr].value == old(Counter[addr].value) + 1;
    }

    // =========================================================================
    // 3. Two-state spec fun with state labels (doc example)
    // =========================================================================

    fun two_increments(addr: address) acquires Counter {
        apply_fn(|a| increment(a) spec {
            modifies Counter[a];
            ensures counter_increased(a);
            ensures Counter[a].value == old(Counter[a].value) + 1;
            aborts_if !exists<Counter>(a);
            aborts_if Counter[a].value + 1 > MAX_U64;
        }, addr);
        apply_fn(|a| increment(a) spec {
            modifies Counter[a];
            ensures counter_increased(a);
            ensures Counter[a].value == old(Counter[a].value) + 1;
            aborts_if !exists<Counter>(a);
            aborts_if Counter[a].value + 1 > MAX_U64;
        }, addr);
    }
    spec two_increments {
        pragma aborts_if_is_partial;
        // First increment: entry → S
        ensures ..S |~ counter_increased(addr);
        // Second increment: S → exit
        ensures S.. |~ counter_increased(addr);
    }

}
