// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Tests behavioral predicates combined with spec functions, including spec
// functions that use `old()` (uses_old) to observe both pre and post states.
//
// When a spec function with uses_old appears in a closure's inline spec or
// in the function spec's ensures clause, the behavioral predicate evaluator
// must propagate the correct dual-state memory parameters.  Frame conditions
// constrain resources that the closure doesn't modify.

module 0x42::spec_functions {
    struct Counter has key { value: u64 }
    struct Config  has key { active: bool }

    // =========================================================================
    // Spec functions
    // =========================================================================

    /// Checks Counter value strictly increased — observes old and current state.
    spec fun counter_increased(addr: address): bool {
        old(Counter[addr].value) < Counter[addr].value
    }

    /// Pure spec function: checks counter in current state only.
    spec fun counter_is_positive(addr: address): bool {
        Counter[addr].value > 0
    }

    // =========================================================================
    // Library functions
    // =========================================================================

    /// Opaque increment: modifies Counter, reads Config.
    fun increment_if_active(addr: address) acquires Counter, Config {
        if (Config[addr].active) {
            Counter[addr].value = Counter[addr].value + 1;
        };
    }
    spec increment_if_active {
        pragma opaque;
        pragma aborts_if_is_partial;
        modifies Counter[addr];
        ensures Config[addr].active ==> counter_increased(addr);
    }

    // =========================================================================
    // Opaque higher-order wrapper
    // =========================================================================

    fun apply(f: |address|, x: address) {
        f(x)
    }
    spec apply {
        pragma opaque;
        reads_of<f> Config;
        modifies_of<f>(a: address) Counter[a];
        ensures ensures_of<f>(x);
        aborts_if aborts_of<f>(x);
    }

    // =========================================================================
    // 1. Spec function with uses_old in closure spec — success
    // =========================================================================

    /// The inline spec uses counter_increased (uses_old).
    /// Config is read-only, so it is unchanged at the opaque call site.
    fun test_uses_old_in_closure(addr: address) acquires Counter, Config {
        apply(|a| increment_if_active(a) spec {
            modifies Counter[a];
            ensures Config[a].active ==> counter_increased(a);
        }, addr);
    }
    spec test_uses_old_in_closure {
        pragma aborts_if_is_partial;
        ensures Config[addr] == old(Config[addr]);
    }

    // =========================================================================
    // 2. Spec function with uses_old — wrong result claim
    // =========================================================================

    /// The ensures_of says counter_increased when active, but here we claim
    /// the counter is always positive (which isn't guaranteed by the spec).
    fun test_spec_fun_wrong_claim(addr: address) acquires Counter, Config {
        apply(|a| increment_if_active(a) spec {
            modifies Counter[a];
            ensures Config[a].active ==> counter_increased(a);
        }, addr);
    }
    spec test_spec_fun_wrong_claim {
        pragma aborts_if_is_partial;
        ensures counter_is_positive(addr); // error: not guaranteed by spec
    }

    // =========================================================================
    // 3. Multiple uses_old spec functions — success
    // =========================================================================

    spec fun counter_non_negative(addr: address): bool {
        Counter[addr].value >= 0  // always true for u64, but tests spec fun handling
    }

    /// Using a non-old spec function alongside ensures_of with old spec fun.
    fun test_mixed_spec_funs(addr: address) acquires Counter, Config {
        apply(|a| increment_if_active(a) spec {
            modifies Counter[a];
            ensures Config[a].active ==> counter_increased(a);
        }, addr);
    }
    spec test_mixed_spec_funs {
        pragma aborts_if_is_partial;
        // counter_non_negative is trivially true for u64
        ensures counter_non_negative(addr);
        ensures Config[addr] == old(Config[addr]);
    }

    // =========================================================================
    // 4. Transitive spec function memory — spec fun calling spec fun
    // =========================================================================

    /// Wrapper spec function that does NOT directly reference Counter,
    /// but delegates to counter_is_positive which does.
    spec fun counter_ok(addr: address): bool {
        counter_is_positive(addr)
    }

    /// Opaque read-only function returning a Counter value.
    fun get_counter(addr: address): u64 acquires Counter {
        Counter[addr].value
    }
    spec get_counter {
        pragma opaque;
        ensures result == Counter[addr].value;
        aborts_if !exists<Counter>(addr);
    }

    fun apply_ret(f: |address| u64, x: address): u64 {
        f(x)
    }
    spec apply_ret {
        pragma opaque;
        reads_of<f> Counter, Config;
        ensures ensures_of<f>(x, result);
        aborts_if aborts_of<f>(x);
    }

    /// Use `counter_ok` (transitive) in the closure spec ensures.
    /// Counter must be discovered transitively through counter_ok → counter_is_positive.
    fun test_transitive_spec_fun(addr: address): u64 acquires Counter {
        apply_ret(|a| get_counter(a) spec {
            ensures result == Counter[a].value;
            aborts_if !exists<Counter>(a);
        }, addr)
    }
    spec test_transitive_spec_fun {
        pragma aborts_if_is_partial;
        // counter_ok is transitive (calls counter_is_positive which reads Counter).
        // If memory discovery didn't follow the spec function call chain, Counter
        // would be missing from the memory footprint and Boogie would fail.
        ensures counter_ok(addr) ==> result > 0;
    }
}
