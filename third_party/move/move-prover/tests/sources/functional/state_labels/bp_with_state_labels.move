// Copyright © Aptos Foundation
// Tests that behavioral predicate functions ($bp_aborts_of, $bp_ensures_of)
// correctly constrain existentially quantified intermediate memory labels.
//
// Bug: when a function spec uses state labels (e.g., ..S |~ remove, S.. |~ publish),
// the $bp_aborts_of function wraps the intermediate label in an existential but
// WITHOUT including the frame constraints from ensures conditions that define
// what those labels mean.
module 0x42::bp_with_state_labels {
    use std::signer::address_of;

    struct R has key, drop { v: u64 }

    // =========================================================================
    // Opaque function with state-labeled spec (remove + publish)
    // =========================================================================

    /// Removes R then re-publishes with incremented value.
    fun replace(s: &signer) {
        let addr = address_of(s);
        let R { v } = move_from<R>(addr);
        move_to(s, R { v: v + 1 });
    }
    spec replace {
        pragma opaque;
        modifies R[address_of(s)];
        ensures ..S |~ remove<R>(address_of(s));
        ensures S.. |~ publish<R>(address_of(s),
            update_field(old(R[address_of(s)]), v, old(R[address_of(s)].v) + 1));
        aborts_if !exists<R>(address_of(s));
        aborts_if S |~ exists<R>(address_of(s));
        aborts_if R[address_of(s)].v + 1 > MAX_U64;
    }

    // =========================================================================
    // Caller that directly invokes replace and verifies abort behavior
    // =========================================================================

    /// Wraps replace. Its spec uses aborts_of<replace> indirectly through
    /// the opaque spec of replace.
    fun test_replace(s: &signer) {
        replace(s);
    }
    spec test_replace {
        // After the remove at S, R doesn't exist (by ..S |~ remove),
        // so "S |~ exists<R>(addr)" is false. This abort should not fire.
        aborts_if !exists<R>(address_of(s));
        aborts_if R[address_of(s)].v + 1 > MAX_U64;
    }
}
