// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Comprehensive tests for frame conditions in behavioral predicates.
//
// When an opaque higher-order function dispatches a closure call, the apply
// procedure havocs ALL memory then constrains results via behavioral predicates.
// Frame conditions ensure that resources not written by the closure are unchanged.
//
// Access classification:
//   Reads      — no modifies clause → post == pre
//   WritesAt   — modifies R[a] → unchanged except at address a
//   WritesAll  — non-opaque closure → no frame constraint
//
// NOTE: At an opaque call site, memory is NOT havoc'd — frame conditions are
// exercised inside the apply procedure during verification of the HO function.
// At the caller level, memory is unchanged, so frame-related postconditions
// hold trivially.  The ensures_of assumption constrains return values.

module 0x42::frame_conditions {

    // =========================================================================
    // Resources
    // =========================================================================

    struct Counter has key { value: u64 }
    struct Config  has key { active: bool }
    struct Balance has key { coins: u64 }

    // =========================================================================
    // Library: opaque closures that read / write resources
    // =========================================================================

    /// Reads Counter only (opaque, no modifies).
    fun get_counter(addr: address): u64 acquires Counter {
        Counter[addr].value
    }
    spec get_counter {
        pragma opaque;
        ensures result == Counter[addr].value;
        aborts_if !exists<Counter>(addr);
    }

    /// Reads Counter only — NOT opaque (WritesAll path).
    fun get_counter_transparent(addr: address): u64 acquires Counter {
        Counter[addr].value
    }
    spec get_counter_transparent {
        ensures result == Counter[addr].value;
        aborts_if !exists<Counter>(addr);
    }

    /// Reads Counter and Config, returns conditional result (opaque, no modifies).
    fun get_conditional(addr: address): u64 acquires Counter, Config {
        if (Config[addr].active) { Counter[addr].value } else { 0 }
    }
    spec get_conditional {
        pragma opaque;
        pragma aborts_if_is_partial;
        ensures Config[addr].active ==> result == Counter[addr].value;
        ensures !Config[addr].active ==> result == 0;
    }

    /// Modifies Balance at a specific address, reads Config (opaque).
    fun increment_balance(addr: address) acquires Balance, Config {
        if (Config[addr].active) {
            Balance[addr].coins = Balance[addr].coins + 1;
        };
    }
    spec increment_balance {
        pragma opaque;
        pragma aborts_if_is_partial;
        modifies Balance[addr];
        aborts_if !exists<Config>(addr);
    }

    // =========================================================================
    // Higher-order opaque wrappers
    // =========================================================================

    fun apply(f: |address| u64, x: address): u64 {
        f(x)
    }
    spec apply {
        pragma opaque;
        reads_of<f> Counter, Config;
        ensures ensures_of<f>(x, result);
        aborts_if aborts_of<f>(x);
    }

    fun apply_void(f: |address|, x: address) {
        f(x)
    }
    spec apply_void {
        pragma opaque;
        reads_of<f> Config;
        modifies_of<f>(a: address) Balance[a];
        ensures ensures_of<f>(x);
        aborts_if aborts_of<f>(x);
    }

    // =========================================================================
    // 1. Reads — opaque read-only closure: memory unchanged
    // =========================================================================

    /// Counter is unchanged after calling a read-only opaque closure.
    fun test_reads_frame(addr: address): u64 acquires Counter {
        apply(|a| get_counter(a) spec {
            ensures result == Counter[a].value;
            aborts_if !exists<Counter>(a);
        }, addr)
    }
    spec test_reads_frame {
        pragma aborts_if_is_partial;
        ensures result == Counter[addr].value;
        ensures Counter[addr] == old(Counter[addr]);
    }

    /// Both Counter and Config are unchanged after a read-only closure.
    fun test_reads_multiple_resources(addr: address): u64 acquires Counter, Config {
        apply(|a| get_conditional(a) spec {
            ensures Config[a].active ==> result == Counter[a].value;
            ensures !Config[a].active ==> result == 0;
        }, addr)
    }
    spec test_reads_multiple_resources {
        pragma aborts_if_is_partial;
        ensures Counter[addr] == old(Counter[addr]);
        ensures Config[addr] == old(Config[addr]);
    }

    // =========================================================================
    // 2. aborts_of — success
    // =========================================================================

    /// aborts_of propagates the abort condition from the closure.
    fun test_aborts_of(addr: address): u64 acquires Counter {
        apply(|a| get_counter(a) spec {
            ensures result == Counter[a].value;
            aborts_if !exists<Counter>(a);
        }, addr)
    }
    spec test_aborts_of {
        aborts_if !exists<Counter>(addr);
    }

    /// The closure only aborts when Counter is missing, not always.
    fun test_aborts_of_wrong(addr: address): u64 acquires Counter {
        apply(|a| get_counter(a) spec {
            ensures result == Counter[a].value;
            aborts_if !exists<Counter>(a);
        }, addr)
    }
    spec test_aborts_of_wrong {
        aborts_if true; // error: abort not covered by aborts_if
    }

    // =========================================================================
    // 3. Wrong postcondition — failure
    // =========================================================================

    /// Wrong result: the closure returns Counter.value, not Counter.value + 1.
    fun test_reads_frame_wrong_result(addr: address): u64 acquires Counter {
        apply(|a| get_counter(a) spec {
            ensures result == Counter[a].value;
            aborts_if !exists<Counter>(a);
        }, addr)
    }
    spec test_reads_frame_wrong_result {
        pragma aborts_if_is_partial;
        ensures result == Counter[addr].value + 1; // error: post-condition does not hold
    }

    // =========================================================================
    // 4. WritesAt — only the specified address can change
    // =========================================================================

    /// After increment_balance(addr), Config is unchanged (reads-only)
    /// and Balance at other addresses is unchanged (writes-at frame).
    fun test_writes_at_frame(addr: address) acquires Balance, Config {
        apply_void(|a| increment_balance(a) spec {
            modifies Balance[a];
            aborts_if !exists<Config>(a);
        }, addr);
    }
    spec test_writes_at_frame {
        pragma aborts_if_is_partial;
        ensures Config[addr] == old(Config[addr]);
        ensures forall a: address where a != addr:
            Balance[a] == old(Balance[a]);
    }

    // =========================================================================
    // 5. WritesAll — non-opaque closure: result still correct
    // =========================================================================

    /// Result is correct via ensures_of even without frame conditions.
    fun test_writes_all_result(addr: address): u64 acquires Counter {
        apply(|a| get_counter_transparent(a) spec {
            ensures result == Counter[a].value;
            aborts_if !exists<Counter>(a);
        }, addr)
    }
    spec test_writes_all_result {
        pragma aborts_if_is_partial;
        ensures result == Counter[addr].value;
    }

    // =========================================================================
    // 6. result_of with frame conditions — success
    // =========================================================================

    fun apply_result(f: |address| u64, x: address): u64 {
        f(x)
    }
    spec apply_result {
        pragma opaque;
        pragma verify = false;
        reads_of<f> Counter, Config;
        ensures result == result_of<f>(x);
        ensures ensures_of<f>(x, result);
        aborts_if aborts_of<f>(x);
    }

    /// result_of with a read-only opaque closure: both resources unchanged.
    fun test_result_of_frame(addr: address): u64 acquires Counter, Config {
        apply_result(|a| get_conditional(a) spec {
            ensures Config[a].active ==> result == Counter[a].value;
            ensures !Config[a].active ==> result == 0;
        }, addr)
    }
    spec test_result_of_frame {
        pragma aborts_if_is_partial;
        ensures Counter[addr] == old(Counter[addr]);
        ensures Config[addr] == old(Config[addr]);
    }
}
