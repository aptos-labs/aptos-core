// Copyright © Aptos Foundation

// flag: --check-inconsistency
// flag: --unconditional-abort-as-inconsistency

// A behavioral predicate under `old(..)` is still evaluated at the post-state
// memory (its memory arguments do not follow `old`), so a clause containing
// one is not pre-state-only and must not enter the unguarded relation of the
// carrier: at a pair which is no transition of the callee, the post-state
// read would be forced to the pre-state value.
module 0x42::result_of_transition_old_bp {
    struct R has key { v: u64 }
    struct T has key { v: u64 }

    fun read_t(a: address): u64 acquires T {
        T[a].v
    }
    spec read_t {
        pragma opaque;
        aborts_if !exists<T>(a);
        ensures result == global<T>(a).v;
    }

    // Writes `R` only; the clause over `read_t` is about `T` at entry.
    fun bump(a: address): u64 acquires R {
        let r = &mut R[a];
        let prev = r.v;
        r.v = r.v + 1;
        prev
    }
    spec bump {
        pragma opaque;
        requires exists<T>(a);
        requires global<T>(a).v == 7;
        aborts_if !exists<R>(a);
        aborts_if global<R>(a).v + 1 > MAX_U64;
        ensures result == old(global<R>(a).v);
        ensures old(result_of<read_t>(a)) == 7;
        ensures global<R>(a).v == old(global<R>(a).v) + 1;
        modifies global<R>(a);
    }

    // Canary: must not verify. The carrier's pair (entry, exit) is no
    // transition of `bump`: `T` is written after the call.
    fun caller_false(a: address): u64 acquires R, T {
        let prev = bump(a);
        T[a].v = 8;
        prev
    }
    spec caller_false {
        pragma aborts_if_is_partial;
        requires exists<R>(a);
        requires exists<T>(a);
        requires global<T>(a).v == 7;
        requires global<R>(a).v < 100;
        ensures result == result_of<bump>(a);
        ensures false;
    }

    // Legitimate consumer across the call.
    fun caller_tied(a: address): u64 acquires R {
        bump(a)
    }
    spec caller_tied {
        pragma aborts_if_is_partial;
        requires exists<R>(a);
        requires exists<T>(a);
        requires global<T>(a).v == 7;
        requires global<R>(a).v < 100;
        ensures result == result_of<bump>(a);
        ensures result == old(global<R>(a).v);
        ensures global<R>(a).v == old(global<R>(a).v) + 1;
    }
}
