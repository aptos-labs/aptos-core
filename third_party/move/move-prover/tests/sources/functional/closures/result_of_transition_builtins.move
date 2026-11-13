// Copyright © Aptos Foundation

// flag: --check-inconsistency
// flag: --unconditional-abort-as-inconsistency

// A postcondition stated with a mutation builtin (`remove`, `publish`,
// `update`) constrains the post-state memory as much as a clause reading it
// does: it must not enter the unguarded pre-state-only relation of the
// carrier, or the carrier at a single state forces the callee's abort
// condition again.
module 0x42::result_of_transition_builtins {
    use std::signer;

    struct R has key { v: u64 }

    fun take(a: address): u64 acquires R {
        let R { v } = move_from<R>(a);
        v
    }
    spec take {
        pragma opaque;
        aborts_if !exists<R>(a);
        ensures remove<R>(a);
        ensures result == old(global<R>(a).v);
    }

    // Canary: must not verify.
    fun caller_take_false(a: address): u64 acquires R {
        take(a)
    }
    spec caller_take_false {
        pragma aborts_if_is_partial;
        requires exists<R>(a);
        aborts_if result_of<take>(a) > MAX_U64;
        ensures false;
    }

    fun put(s: &signer): u64 {
        move_to(s, R { v: 1 });
        1
    }
    spec put {
        pragma opaque;
        aborts_if exists<R>(signer::address_of(s));
        ensures publish<R>(signer::address_of(s), R { v: 1 });
        ensures result == 1;
    }

    // Canary: must not verify.
    fun caller_put_false(s: &signer): u64 {
        put(s)
    }
    spec caller_put_false {
        pragma aborts_if_is_partial;
        requires !exists<R>(signer::address_of(s));
        aborts_if result_of<put>(s) > MAX_U64;
        ensures false;
    }

    fun set(a: address): u64 acquires R {
        R[a].v = 5;
        5
    }
    spec set {
        pragma opaque;
        aborts_if !exists<R>(a);
        ensures update<R>(a, R { v: 5 });
        ensures result == 5;
    }

    // Canary: must not verify.
    fun caller_set_false(a: address): u64 acquires R {
        set(a)
    }
    spec caller_set_false {
        pragma aborts_if_is_partial;
        requires exists<R>(a);
        requires global<R>(a).v == 7;
        aborts_if result_of<set>(a) > MAX_U64;
        ensures false;
    }

    // Legitimate consumer across the call.
    fun caller_set_tied(a: address): u64 acquires R {
        set(a)
    }
    spec caller_set_tied {
        pragma aborts_if_is_partial;
        requires exists<R>(a);
        ensures ensures_of<set>(a, result_of<set>(a)) && result == result_of<set>(a);
        ensures global<R>(a).v == 5;
    }
}
