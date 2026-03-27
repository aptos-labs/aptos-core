// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Tests that spec function calls at state labels correctly resolve memory for
// resources NOT modified by the label's defining operation.  Without label
// resolution, a reference like `Counter$memory#S` (where S only modified Flag)
// would be unconstrained in the generated Boogie, leading to unsound results.

module 0x42::spec_fun_cross_resource {
    struct Counter has key { value: u64 }
    struct Flag has key { active: bool }

    // =========================================================================
    // Spec functions
    // =========================================================================

    /// Pure (non-uses_old): reads Counter at the current state.
    spec fun counter_positive(addr: address): bool {
        Counter[addr].value > 0
    }

    /// Two-state (uses_old): reads Counter in both old and current states.
    spec fun counter_unchanged(addr: address): bool {
        old(Counter[addr].value) == Counter[addr].value
    }

    // =========================================================================
    // Library: opaque primitives
    // =========================================================================

    fun increment(addr: address) acquires Counter {
        Counter[addr].value = Counter[addr].value + 1;
    }
    spec increment {
        pragma opaque;
        modifies Counter[addr];
        ensures Counter[addr].value == old(Counter[addr].value) + 1;
        aborts_if !exists<Counter>(addr);
        aborts_if Counter[addr].value + 1 > MAX_U64;
    }

    fun flip_flag(addr: address) acquires Flag {
        Flag[addr].active = !Flag[addr].active;
    }
    spec flip_flag {
        pragma opaque;
        modifies Flag[addr];
        ensures Flag[addr].active != old(Flag[addr].active);
        aborts_if !exists<Flag>(addr);
    }

    // =========================================================================
    // Single higher-order wrapper (both resources declared)
    // =========================================================================

    fun apply(f: |address|, x: address) {
        f(x)
    }
    spec apply {
        pragma opaque;
        reads_of<f> Counter, Flag;
        modifies_of<f>(a: address) Counter[a], Flag[a];
        ensures ensures_of<f>(x);
        aborts_if aborts_of<f>(x);
    }

    // =========================================================================
    // 1. Non-uses_old spec fun at label where its resource is NOT modified
    // =========================================================================

    /// First: increment Counter (entry → S).
    /// Second: flip Flag (S → exit).
    /// counter_positive reads Counter. In S→exit, only Flag is modified.
    fun test_non_old_cross_resource(addr: address) acquires Counter, Flag {
        apply(|a| increment(a) spec {
            modifies Counter[a];
            ensures Counter[a].value == old(Counter[a].value) + 1;
            aborts_if !exists<Counter>(a);
            aborts_if Counter[a].value + 1 > MAX_U64;
        }, addr);
        apply(|a| flip_flag(a) spec {
            modifies Flag[a];
            ensures Flag[a].active != old(Flag[a].active);
            aborts_if !exists<Flag>(a);
        }, addr);
    }
    spec test_non_old_cross_resource {
        pragma aborts_if_is_partial;
        // Define S via mutation on Counter (fully pins Counter at S).
        ensures ..S |~ update<Counter>(addr,
            update_field(old(Counter[addr]), value, old(Counter[addr].value) + 1));
        // counter_positive reads Counter. In range S→exit, only Flag is modified.
        // Without label resolution, Counter at the S label would be unconstrained.
        requires Counter[addr].value > 0;
        ensures S.. |~ counter_positive(addr);
    }

    // =========================================================================
    // 2. Labeled behavioral predicate where the predicate's resource is NOT
    //    modified at the label (P1 regression test).
    //
    // increment modifies Counter (entry→S1). flip_flag modifies Flag (S1→exit).
    // The spec uses `S1.. |~ ensures_of<flip_flag>(addr)` — flip_flag reads Flag,
    // which was NOT modified at S1. Without label resolution in
    // emit_fun_spec_memory_args, Flag$S1 would be unconstrained in Boogie,
    // making the ensures_of unprovable.
    // =========================================================================

    fun test_behavior_cross_resource(addr: address) acquires Counter, Flag {
        increment(addr);
        flip_flag(addr);
    }
    spec test_behavior_cross_resource {
        pragma aborts_if_is_partial;
        modifies Counter[addr], Flag[addr];
        ensures ..S1 |~ ensures_of<increment>(addr);
        ensures S1.. |~ ensures_of<flip_flag>(addr);
        ensures Counter[addr].value == old(Counter[addr].value) + 1;
        ensures Flag[addr].active != old(Flag[addr].active);
    }

    // =========================================================================
    // 3. uses_old spec fun at label where its resource is NOT modified
    // =========================================================================

    /// Same call sequence: increment Counter then flip Flag.
    /// counter_unchanged reads Counter in both old and current states.
    /// In S→exit, Counter is unchanged, so counter_unchanged should hold.
    fun test_uses_old_cross_resource(addr: address) acquires Counter, Flag {
        apply(|a| increment(a) spec {
            modifies Counter[a];
            ensures Counter[a].value == old(Counter[a].value) + 1;
            aborts_if !exists<Counter>(a);
            aborts_if Counter[a].value + 1 > MAX_U64;
        }, addr);
        apply(|a| flip_flag(a) spec {
            modifies Flag[a];
            ensures Flag[a].active != old(Flag[a].active);
            aborts_if !exists<Flag>(a);
        }, addr);
    }
    spec test_uses_old_cross_resource {
        pragma aborts_if_is_partial;
        // Define S via mutation on Counter.
        ensures ..S |~ update<Counter>(addr,
            update_field(old(Counter[addr]), value, old(Counter[addr].value) + 1));
        // counter_unchanged is uses_old: compares old(Counter) with Counter.
        // In S→exit, Counter is NOT modified (only Flag is).
        // Pre=S, post=exit: Counter at both should be the same value.
        ensures S.. |~ counter_unchanged(addr);
    }
}
