// Copyright © Aptos Foundation

// flag: --check-inconsistency
// flag: --unconditional-abort-as-inconsistency

// A callee writing none of the memory its contract observes still has its
// `result_of` axiom quantified over a pair of states once the contract reads
// the pre-state: a clause relating the two states is false off the diagonal,
// and an unguarded axiom would force the callee's abort condition there,
// letting a writer prove `ensures false` from `result == result_of<get>(a)`.
// Such a callee is transition-guarded unless every `ensures` clause reads the
// pre-state alone; that one is not, and its carrier is still the result of
// the call at any state.

// A two-state frame clause beside the result.
module 0x42::reader_frame {
    struct R has key { v: u64 }

    fun get(a: address): u64 acquires R {
        R[a].v
    }
    spec get {
        pragma opaque;
        aborts_if !exists<R>(a);
        ensures result == global<R>(a).v;
        ensures global<R>(a).v == old(global<R>(a).v);
    }

    // Canaries: must not verify.
    fun caller_ctrl(a: address): u64 acquires R {
        get(a)
    }
    spec caller_ctrl {
        requires exists<R>(a);
        ensures false;
    }

    fun caller_single(a: address): u64 acquires R {
        get(a)
    }
    spec caller_single {
        pragma aborts_if_is_partial;
        requires exists<R>(a);
        aborts_if result_of<get>(a) > MAX_U64;
        ensures false;
    }

    fun caller_two(a: address): u64 acquires R {
        let x = get(a);
        R[a].v = x + 1;
        x
    }
    spec caller_two {
        pragma aborts_if_is_partial;
        requires exists<R>(a);
        requires global<R>(a).v < 100;
        ensures result == result_of<get>(a);
        ensures false;
    }
}

// A pre-state `let` beside a post-state read.
module 0x42::reader_let_pre_post {
    struct R has key { v: u64 }

    fun get(a: address): u64 acquires R {
        R[a].v
    }
    spec get {
        pragma opaque;
        aborts_if !exists<R>(a);
        let pre = global<R>(a).v;
        ensures result == pre;
        ensures result == global<R>(a).v;
    }

    // Canaries: must not verify.
    fun caller_ctrl(a: address): u64 acquires R {
        get(a)
    }
    spec caller_ctrl {
        requires exists<R>(a);
        ensures false;
    }

    fun caller_single(a: address): u64 acquires R {
        get(a)
    }
    spec caller_single {
        pragma aborts_if_is_partial;
        requires exists<R>(a);
        aborts_if result_of<get>(a) > MAX_U64;
        ensures false;
    }

    fun caller_two(a: address): u64 acquires R {
        let x = get(a);
        R[a].v = x + 1;
        x
    }
    spec caller_two {
        pragma aborts_if_is_partial;
        requires exists<R>(a);
        requires global<R>(a).v < 100;
        ensures result == result_of<get>(a);
        ensures false;
    }
}

// A pre-state `let` alone: not guarded, and the carrier is tied to the call.
module 0x42::reader_let_pre {
    struct R has key { v: u64 }

    fun get(a: address): u64 acquires R {
        R[a].v
    }
    spec get {
        pragma opaque;
        aborts_if !exists<R>(a);
        let pre = global<R>(a).v;
        ensures result == pre;
    }

    // Canaries: must not verify.
    fun caller_ctrl(a: address): u64 acquires R {
        get(a)
    }
    spec caller_ctrl {
        requires exists<R>(a);
        ensures false;
    }

    fun caller_single(a: address): u64 acquires R {
        get(a)
    }
    spec caller_single {
        pragma aborts_if_is_partial;
        requires exists<R>(a);
        aborts_if result_of<get>(a) > MAX_U64;
        ensures false;
    }

    fun caller_two(a: address): u64 acquires R {
        let x = get(a);
        R[a].v = x + 1;
        x
    }
    spec caller_two {
        pragma aborts_if_is_partial;
        requires exists<R>(a);
        requires global<R>(a).v < 100;
        ensures result == result_of<get>(a);
        ensures false;
    }

    // The carrier is the result of the call at the pre-state.
    fun caller_tied(a: address): u64 acquires R {
        get(a)
    }
    spec caller_tied {
        requires exists<R>(a);
        ensures result == result_of<get>(a);
    }
}
