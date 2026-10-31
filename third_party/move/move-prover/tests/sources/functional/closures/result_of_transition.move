// Copyright © Aptos Foundation

// flag: --check-inconsistency
// flag: --unconditional-abort-as-inconsistency

// The postcondition of a function which writes memory its contract observes
// holds only across an actual call. The `result_of` axiom relating a callee's
// carrier to its postcondition is guarded by a transition witness which only
// an assumed `ensures_of` of the callee establishes; without the guard, the
// axiom instance at a single state (an `aborts_if` mentioning the carrier)
// contradicts a two-state clause such as `v == old(v) + 1`, which forces the
// callee's abort condition and lets the caller prove `ensures false`.
module 0x42::result_of_transition {
    struct R has key { v: u64 }

    fun bump(a: address): (u64, u64) acquires R {
        let r = borrow_global_mut<R>(a);
        r.v = r.v + 1;
        (1, 2)
    }
    spec bump {
        pragma opaque;
        aborts_if !exists<R>(a);
        aborts_if global<R>(a).v + 1 > MAX_U64;
        ensures result_1 == 1;
        ensures result_2 == 2;
        ensures global<R>(a).v == old(global<R>(a).v) + 1;
        modifies global<R>(a);
    }

    // Canary: the carrier in a single-state position must not make the call
    // site inconsistent.
    fun caller_false(a: address): u64 acquires R {
        let (x, y) = bump(a);
        x + y
    }
    spec caller_false {
        pragma aborts_if_is_partial;
        requires exists<R>(a);
        requires global<R>(a).v < 100;
        aborts_if ({ let (p, q) = result_of<bump>(a); p + q > MAX_U64 });
        ensures false;
    }

    fun bump1(a: address): u64 acquires R {
        let r = borrow_global_mut<R>(a);
        r.v = r.v + 1;
        7
    }
    spec bump1 {
        pragma opaque;
        aborts_if !exists<R>(a);
        aborts_if global<R>(a).v + 1 > MAX_U64;
        ensures result == 7;
        ensures global<R>(a).v == old(global<R>(a).v) + 1;
        modifies global<R>(a);
    }

    // Canary: the single-result carrier, mentioned in an abort condition.
    fun caller1_false(a: address): u64 acquires R {
        bump1(a)
    }
    spec caller1_false {
        pragma aborts_if_is_partial;
        requires exists<R>(a);
        requires global<R>(a).v < 100;
        aborts_if result_of<bump1>(a) > MAX_U64;
        ensures false;
    }

    // A writer whose contract states the written value without `old`: its
    // memory signature has a single slot, at which the unguarded axiom would
    // claim the post-state value of any state the call does not abort in.
    // The axiom evaluates the abort condition at that same slot, so the
    // contract states existence after the call for the consumer below.
    fun set2(a: address): (u64, u64) acquires R {
        borrow_global_mut<R>(a).v = 1;
        (0, 0)
    }
    spec set2 {
        pragma opaque;
        aborts_if !exists<R>(a);
        ensures result_1 == 0;
        ensures result_2 == 0;
        ensures exists<R>(a);
        ensures global<R>(a).v == 1;
        modifies global<R>(a);
    }

    // Canary: must not verify; the entry state has `v == 5`, not `1`.
    fun caller2_false(a: address): u64 acquires R {
        let (x, y) = set2(a);
        x + y
    }
    spec caller2_false {
        pragma aborts_if_is_partial;
        requires exists<R>(a);
        requires global<R>(a).v == 5;
        aborts_if ({ let (p, q) = result_of<set2>(a); p + q > MAX_U64 });
        ensures false;
    }

    // Control: the same body without any carrier mention. Must not verify.
    fun caller_plain(a: address): u64 acquires R {
        let (x, y) = bump(a);
        x + y
    }
    spec caller_plain {
        pragma aborts_if_is_partial;
        requires exists<R>(a);
        requires global<R>(a).v < 100;
        ensures false;
    }

    // Legitimate consumer: across the call, the carrier is the call's result.
    fun caller_tied(a: address): u64 acquires R {
        let (x, y) = bump(a);
        x + y
    }
    spec caller_tied {
        pragma aborts_if_is_partial;
        requires exists<R>(a);
        requires global<R>(a).v < 100;
        ensures ({
            let (p, q) = result_of<bump>(a);
            ensures_of<bump>(a, p, q) && result == p + q
        });
        ensures global<R>(a).v == old(global<R>(a).v) + 1;
    }

    // Legitimate consumer of the single-slot writer.
    fun caller2_tied(a: address): u64 acquires R {
        let (x, y) = set2(a);
        x + y
    }
    spec caller2_tied {
        pragma aborts_if_is_partial;
        requires exists<R>(a);
        ensures ({
            let (p, q) = result_of<set2>(a);
            ensures_of<set2>(a, p, q) && result == p + q
        });
        ensures global<R>(a).v == 1;
    }

    // A function which does not call `bump` cannot state the postcondition
    // of `bump` over its own states (its memory clause is false there); the
    // same claim across the call holds.
    fun no_call(a: address): u64 {
        3
    }
    spec no_call {
        ensures ensures_of<bump>(a, 1, 2);
    }

    fun with_call(a: address): u64 acquires R {
        let (x, y) = bump(a);
        x + y
    }
    spec with_call {
        pragma aborts_if_is_partial;
        requires exists<R>(a);
        requires global<R>(a).v < 100;
        ensures ensures_of<bump>(a, 1, 2);
    }

    // A callee which writes none of the memory its contract observes keeps
    // the unguarded axiom: its result is determined at any state the call
    // does not abort in, whether or not it is called.
    fun check(a: address): (u64, u64) acquires R {
        let _ = borrow_global<R>(a).v;
        (0, 0)
    }
    spec check {
        pragma opaque;
        aborts_if !exists<R>(a);
        ensures result_1 == 0;
        ensures result_2 == 0;
    }

    fun check_no_call(a: address): u64 {
        0
    }
    spec check_no_call {
        ensures exists<R>(a) ==> ({ let (p, q) = result_of<check>(a); p + q == result });
    }

    // For a transition-guarded callee, the clauses of its postcondition which
    // read no post-state memory hold for its result at any state the call
    // does not abort in, so this claim without a call still verifies.
    fun bump_no_call(a: address): u64 {
        3
    }
    spec bump_no_call {
        ensures exists<R>(a) && global<R>(a).v < 100
            ==> ({ let (p, q) = result_of<bump>(a); p + q == result });
    }

    // Premise: the memory clause of the postcondition is available only
    // across a transition. Without a call, the carrier's pair is not one:
    // expected to fail; the fact is only lost, never false.
    fun bump_no_call_memory(a: address): u64 {
        3
    }
    spec bump_no_call_memory {
        ensures exists<R>(a) && global<R>(a).v < 100
            ==> ({ let (p, q) = result_of<bump>(a); global<R>(a).v == old(global<R>(a).v) + 1 });
    }
}
