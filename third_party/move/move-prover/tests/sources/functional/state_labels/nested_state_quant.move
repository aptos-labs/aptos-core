// Copyright © Aptos Foundation
// Tests state-variable binding in data invariants.
//
// A behavioral predicate is state-dependent only when its range is the free
// ambient state (`range.is_default()`). Any state quantifier — forall or exists —
// introduces a bound state variable. After `propagate_state_labels`, `S |~ pred`
// rewrites the Behavior's range to Some(S) regardless of whether S was introduced
// by `forall S in *:` or `exists S in *:`. Only a Behavior whose range is never
// rewritten (i.e., written outside any `S |~` context) is state-dependent.

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
    // Accepted: `exists T in *: T |~ pred` — T is bound by the exists quantifier,
    // aborts_of gets range.pre = Some(T) after propagation, not default.
    // =========================================================================

    struct WithExistsAtTopLevel has key {
        addr: address
    }
    spec WithExistsAtTopLevel {
        invariant exists T in *: T |~ !aborts_of<read_counter>(addr);
    }

    // =========================================================================
    // Accepted: `forall S in *: (S |~ p && exists T in *: T |~ q)` — both bound.
    // =========================================================================

    struct WithNestedExists has key {
        addr: address
    }
    spec WithNestedExists {
        invariant forall S in *:
            (S |~ !aborts_of<read_counter>(addr)) &&
            (exists T in *: T |~ !aborts_of<read_counter>(addr));
    }

    // =========================================================================
    // Rejected: Behavior at the free ambient state (no enclosing `S |~`).
    // =========================================================================

    struct WithFreeStateBehavior has key {
        addr: address
    }
    spec WithFreeStateBehavior {
        invariant !aborts_of<read_counter>(addr); // error: data invariant must not depend on a free state
    }

    // =========================================================================
    // Regression guard: simple forall still accepted.
    // =========================================================================

    struct WithSimpleForall has key {
        addr: address
    }
    spec WithSimpleForall {
        invariant forall S in *: S |~ !aborts_of<read_counter>(addr);
    }
}
