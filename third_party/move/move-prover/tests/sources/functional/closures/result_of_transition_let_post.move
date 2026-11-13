// Copyright © Aptos Foundation

// flag: --check-inconsistency
// flag: --unconditional-abort-as-inconsistency

// The clauses of a transition-guarded callee's postcondition which read no
// post-state memory relate its `result_of` carrier unguarded. A clause is
// judged with only the spec `let`s it references: an unrelated `let post`
// does not disqualify it, and one it references, directly or through another
// `let`, does.
module 0x42::result_of_transition_let_post {
    struct R has key { v: u64 }

    fun bump(a: address): u64 acquires R {
        let r = borrow_global_mut<R>(a);
        r.v = r.v + 1;
        1
    }
    spec bump {
        pragma opaque;
        let post x = global<R>(a).v;
        aborts_if !exists<R>(a);
        aborts_if global<R>(a).v + 1 > MAX_U64;
        ensures x == old(global<R>(a).v) + 1;
        ensures result == 1;
        modifies global<R>(a);
    }

    // Without a call, the carrier is related to `result == 1`.
    fun bump_no_call(a: address): u64 {
        1
    }
    spec bump_no_call {
        ensures exists<R>(a) && global<R>(a).v < 100 ==> result_of<bump>(a) == result;
    }

    // `result == x` reads the post-state through `x`; at a single state it
    // contradicts `result == old(v) + 1`.
    fun bump_direct(a: address): u64 acquires R {
        let r = borrow_global_mut<R>(a);
        r.v = r.v + 1;
        r.v
    }
    spec bump_direct {
        pragma opaque;
        let post x = global<R>(a).v;
        aborts_if !exists<R>(a);
        aborts_if global<R>(a).v + 1 > MAX_U64;
        ensures result == x;
        ensures result == old(global<R>(a).v) + 1;
        modifies global<R>(a);
    }

    // Canary: must not verify (the post-state clause is not admitted).
    fun direct_false(a: address): u64 {
        0
    }
    spec direct_false {
        pragma aborts_if_is_partial;
        requires exists<R>(a);
        requires global<R>(a).v < 100;
        aborts_if result_of<bump_direct>(a) > MAX_U64;
        ensures false;
    }

    // As above, with the post-state read reached through another `let`.
    fun bump_transitive(a: address): u64 acquires R {
        let r = borrow_global_mut<R>(a);
        r.v = r.v + 1;
        r.v
    }
    spec bump_transitive {
        pragma opaque;
        let post y = global<R>(a).v;
        let post x = y;
        aborts_if !exists<R>(a);
        aborts_if global<R>(a).v + 1 > MAX_U64;
        ensures result == x;
        ensures result == old(global<R>(a).v) + 1;
        modifies global<R>(a);
    }

    // Canary: must not verify.
    fun transitive_false(a: address): u64 {
        0
    }
    spec transitive_false {
        pragma aborts_if_is_partial;
        requires exists<R>(a);
        requires global<R>(a).v < 100;
        aborts_if result_of<bump_transitive>(a) > MAX_U64;
        ensures false;
    }
}
