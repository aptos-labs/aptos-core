// Copyright (c) Aptos Foundation
// Tests for state-domain quantification: `forall S in *: S |~ P`
//
// State-domain quantification allows universally quantifying over all
// possible memory states, enabling properties like "f never aborts in
// any state" rather than just "f doesn't abort in the current state".

module 0x42::state_domain_quant {

    struct Counter has key { value: u64 }

    // =========================================================================
    // Opaque helper functions
    // =========================================================================

    fun read_counter(addr: address): u64 acquires Counter {
        Counter[addr].value
    }
    spec read_counter {
        pragma opaque;
        ensures result == Counter[addr].value;
        aborts_if !exists<Counter>(addr);
    }

    fun inc_counter(addr: address) acquires Counter {
        let c = &mut Counter[addr];
        c.value = c.value + 1;
    }
    spec inc_counter {
        pragma opaque;
        modifies Counter[addr];
        ensures Counter[addr].value == old(Counter[addr].value) + 1;
        aborts_if !exists<Counter>(addr);
        aborts_if Counter[addr].value + 1 > MAX_U64;
    }

    // =========================================================================
    // Higher-order wrappers with state-domain quantification
    // =========================================================================

    /// Apply a read-only function. Its spec states that for ANY state,
    /// the function does not abort when Counter exists with a valid value.
    fun apply_reader(
        f: |address| u64,
        addr: address,
    ): u64 {
        f(addr)
    }
    spec apply_reader {
        pragma opaque;
        reads_of<f> Counter;
        modifies_of<f> *;
        // Universal state quantification: f does not abort in ANY state
        // where Counter exists at addr.
        ensures ensures_of<f>(addr, result);
        aborts_if aborts_of<f>(addr);
    }

    /// Test: call apply_reader with read_counter.
    fun test_reader(addr: address): u64 acquires Counter {
        apply_reader(read_counter, addr)
    }
    spec test_reader {
        ensures result == Counter[addr].value;
        aborts_if !exists<Counter>(addr);
    }

    // =========================================================================
    // Error cases: unused state-domain variable
    // =========================================================================

    fun dummy_pure(x: u64): bool { x > 0 }
    spec dummy_pure {
        ensures result == (x > 0);
    }

    fun test_unused_state_label() {
        let _ = dummy_pure(1);
    }
    spec test_unused_state_label {
        // Error: S is not used as a state label in the body
        requires forall S in *, x: u64: x > 0; // error: unused quantified state label
    }
}
