// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

module 0x42::closure_frame_widening {
    struct Counter has key { value: u64 }
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

    struct Flag has key { active: bool }
    fun apply(f: |address|, addr: address) { f(addr) }
    spec apply {
        pragma opaque;
        reads_of<f> Counter, Flag;
        modifies_of<f>(a: address) Counter[a], Flag[a];
        ensures ensures_of<f>(addr);
        aborts_if aborts_of<f>(addr);
    }
    // The callback writes one resource; the wrapper permits two.
    fun caller(addr: address) acquires Counter {
        apply(|a| increment(a), addr);
    }
    spec caller {
        pragma aborts_if_is_partial;
        ensures Counter[addr].value == old(Counter[addr].value) + 1;
    }
    fun increment_by(delta: u64, addr: address) acquires Counter {
        Counter[addr].value = Counter[addr].value + delta;
    }
    spec increment_by {
        pragma opaque;
        modifies Counter[addr];
        ensures Counter[addr].value == old(Counter[addr].value) + delta;
        aborts_if !exists<Counter>(addr);
        aborts_if Counter[addr].value + delta > MAX_U64;
    }
    // Captured arguments must be composed before comparing the two frames.
    fun caller_captured(addr: address, delta: u64) acquires Counter {
        apply(|a| increment_by(delta, a), addr);
    }
    spec caller_captured {
        pragma aborts_if_is_partial;
        ensures Counter[addr].value == old(Counter[addr].value) + delta;
    }
    fun apply_counter(f: |address|, addr: address) { f(addr) }
    spec apply_counter {
        pragma opaque;
        reads_of<f> Counter, Flag;
        modifies_of<f>(a: address) Counter[a];
        ensures ensures_of<f>(addr);
        aborts_if aborts_of<f>(addr);
    }
    // This callback writes a captured address, outside the invocation frame.
    fun wrong_frame(addr: address, other: address) acquires Counter {
        apply_counter(|_a| increment(other), addr);
    }
    spec wrong_frame { pragma aborts_if_is_partial; }
}
