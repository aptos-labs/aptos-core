// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Tests behavioral predicates over sequences of opaque calls, where each call
// produces an intermediate global state.  The key challenge: after each opaque
// higher-order call, the ensures_of assumption constrains the result while
// memory at the caller level remains unchanged (opaque call semantics).
//
// We test:
// - Sequential calls: read after write, using intermediate results
// - aborts_of on intermediate state
// - Combining ensures_of and aborts_of across multiple calls
// - Failure cases with wrong postconditions

module 0x42::intermediate_states {
    struct Counter has key { value: u64 }
    struct Config  has key { active: bool }

    // =========================================================================
    // Library: opaque primitives
    // =========================================================================

    /// Read Counter value (opaque, no modifies).
    fun get_counter(addr: address): u64 acquires Counter {
        Counter[addr].value
    }
    spec get_counter {
        pragma opaque;
        ensures result == Counter[addr].value;
        aborts_if !exists<Counter>(addr);
    }

    /// Opaque identity that reads Config to decide.
    fun get_conditional(addr: address): u64 acquires Counter, Config {
        if (Config[addr].active) { Counter[addr].value } else { 0 }
    }
    spec get_conditional {
        pragma opaque;
        pragma aborts_if_is_partial;
        ensures Config[addr].active ==> result == Counter[addr].value;
        ensures !Config[addr].active ==> result == 0;
    }

    // =========================================================================
    // Higher-order opaque wrappers
    // =========================================================================

    fun apply_read(f: |address| u64, x: address): u64 {
        f(x)
    }
    spec apply_read {
        pragma opaque;
        reads_of<f> Counter, Config;
        ensures ensures_of<f>(x, result);
        aborts_if aborts_of<f>(x);
    }

    // =========================================================================
    // 1. Sequential reads — success
    // =========================================================================

    /// Two sequential opaque reads return the same value (memory unchanged).
    fun test_two_reads(addr: address): (u64, u64) acquires Counter {
        let a = apply_read(|a| get_counter(a) spec {
            ensures result == Counter[a].value;
            aborts_if !exists<Counter>(a);
        }, addr);
        let b = apply_read(|a| get_counter(a) spec {
            ensures result == Counter[a].value;
            aborts_if !exists<Counter>(a);
        }, addr);
        (a, b)
    }
    spec test_two_reads {
        pragma aborts_if_is_partial;
        // Both reads return the same value since memory is unchanged.
        ensures result_1 == result_2;
        ensures result_1 == Counter[addr].value;
    }

    // =========================================================================
    // 2. Sequential reads — failure
    // =========================================================================

    /// Claiming the two reads differ is wrong.
    fun test_two_reads_differ(addr: address): (u64, u64) acquires Counter {
        let a = apply_read(|a| get_counter(a) spec {
            ensures result == Counter[a].value;
            aborts_if !exists<Counter>(a);
        }, addr);
        let b = apply_read(|a| get_counter(a) spec {
            ensures result == Counter[a].value;
            aborts_if !exists<Counter>(a);
        }, addr);
        (a, b)
    }
    spec test_two_reads_differ {
        pragma aborts_if_is_partial;
        ensures result_1 != result_2; // error: both reads return the same value
    }

    // =========================================================================
    // 3. Sequential reads with different closures — success
    // =========================================================================

    /// Read Counter then read conditional — both see same memory.
    fun test_read_then_conditional(addr: address): (u64, u64) acquires Counter, Config {
        let raw = apply_read(|a| get_counter(a) spec {
            ensures result == Counter[a].value;
            aborts_if !exists<Counter>(a);
        }, addr);
        let cond = apply_read(|a| get_conditional(a) spec {
            ensures Config[a].active ==> result == Counter[a].value;
            ensures !Config[a].active ==> result == 0;
        }, addr);
        (raw, cond)
    }
    spec test_read_then_conditional {
        pragma aborts_if_is_partial;
        // When active, both return the same Counter value.
        ensures Config[addr].active ==> result_1 == result_2;
        // When inactive, conditional returns 0.
        ensures !Config[addr].active ==> result_2 == 0;
    }

    // =========================================================================
    // 4. Sequential reads — wrong relationship claim
    // =========================================================================

    /// Claiming conditional always equals raw is wrong (fails when inactive).
    fun test_conditional_always_equals(addr: address): (u64, u64) acquires Counter, Config {
        let raw = apply_read(|a| get_counter(a) spec {
            ensures result == Counter[a].value;
            aborts_if !exists<Counter>(a);
        }, addr);
        let cond = apply_read(|a| get_conditional(a) spec {
            ensures Config[a].active ==> result == Counter[a].value;
            ensures !Config[a].active ==> result == 0;
        }, addr);
        (raw, cond)
    }
    spec test_conditional_always_equals {
        pragma aborts_if_is_partial;
        ensures result_1 == result_2; // error: conditional returns 0 when inactive
    }

    // =========================================================================
    // 5. aborts_of across sequential calls — success
    // =========================================================================

    /// Both calls can abort; the overall function aborts if either does.
    fun test_abort_propagation(addr: address): (u64, u64) acquires Counter, Config {
        let a = apply_read(|a| get_counter(a) spec {
            ensures result == Counter[a].value;
            aborts_if !exists<Counter>(a);
        }, addr);
        let b = apply_read(|a| get_conditional(a) spec {
            ensures Config[a].active ==> result == Counter[a].value;
            ensures !Config[a].active ==> result == 0;
        }, addr);
        (a, b)
    }
    spec test_abort_propagation {
        // Either Counter or Config missing causes abort.
        aborts_if !exists<Counter>(addr);
    }

    // =========================================================================
    // 6. Config preserved across multiple calls — success
    // =========================================================================

    /// Config is read-only in all closures, so it's unchanged across the
    /// entire sequence.
    fun test_config_preserved(addr: address): (u64, u64) acquires Counter, Config {
        let a = apply_read(|a| get_conditional(a) spec {
            ensures Config[a].active ==> result == Counter[a].value;
            ensures !Config[a].active ==> result == 0;
        }, addr);
        let b = apply_read(|a| get_conditional(a) spec {
            ensures Config[a].active ==> result == Counter[a].value;
            ensures !Config[a].active ==> result == 0;
        }, addr);
        (a, b)
    }
    spec test_config_preserved {
        pragma aborts_if_is_partial;
        // Config unchanged across both calls.
        ensures Config[addr] == old(Config[addr]);
        // Both calls see the same state, so results match.
        ensures result_1 == result_2;
    }
}
