// Copyright © Aptos Foundation
// Tests nested state quantifiers in data invariants.
//
// A `forall S in *:` binder universally quantifies the state variable S. A
// non-forall state quantifier (`exists T in *:`, `choose T in *:`) nested
// INSIDE such a binder is bound by it and must be accepted; the same
// quantifier at the TOP LEVEL (free state) must be rejected.

module 0x42::nested_state_quant {

    struct Counter has key { value: u64 }

    fun read_counter(addr: address): u64 acquires Counter {
        Counter[addr].value
    }
    spec read_counter {
        pragma opaque;
        ensures result == Counter[addr].value;
        aborts_if !exists<Counter>(addr);
    }

    // =========================================================================
    // Accepted: `exists T in *:` sibling to `S |~ pred` inside a forall-state
    //           binder — the state variable T is bound by the exists quantifier,
    //           so it is NOT a free state reference.
    // =========================================================================

    // Data invariant: "for all states S, at S, read_counter doesn't abort (i.e.,
    // Counter exists at addr at S), AND there exists a state T where read_counter
    // doesn't abort at T either." Both clauses are inside a forall-state binder so
    // no free state reference escapes.
    struct WithNestedExists has key {
        addr: address
    }
    spec WithNestedExists {
        invariant forall S in *:
            (S |~ !aborts_of<read_counter>(addr)) &&
            (exists T in *: T |~ !aborts_of<read_counter>(addr));
    }

    // =========================================================================
    // Rejected: `exists T in *:` at the top level (free state) — error expected.
    // =========================================================================

    struct WithFreeExists has key {
        addr: address
    }
    spec WithFreeExists {
        // error: data invariant must not depend on a free state
        invariant exists T in *: T |~ !aborts_of<read_counter>(addr);
    }

    // =========================================================================
    // Regression guard: simple forall without nested exists still accepted.
    // =========================================================================

    struct WithSimpleForall has key {
        addr: address
    }
    spec WithSimpleForall {
        invariant forall S in *: S |~ !aborts_of<read_counter>(addr);
    }
}
