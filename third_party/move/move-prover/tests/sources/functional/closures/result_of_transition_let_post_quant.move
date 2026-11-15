// Copyright © Aptos Foundation

// flag: --check-inconsistency
// flag: --unconditional-abort-as-inconsistency
// The Verex lowering resolves a range in the scope of its own binder
// (VEREX-LOCAL-NAME on the shadowed `n`).
// exclude_for: lean

// A spec `let` referenced from a quantifier range is in scope of the clause:
// the range is evaluated before its own binder, so the `n` of `0..n` is the
// post-state `let`, which the binder then shadows in the body. The clause
// reads the post-state and is kept out of the unguarded pre-state relation
// of the `result_of` carrier.
module 0x42::result_of_transition_let_post_quant {
    struct R has key { v: u64 }

    fun bump(a: address): u64 acquires R {
        let r = borrow_global_mut<R>(a);
        r.v = r.v + 1;
        r.v
    }
    spec bump {
        pragma opaque;
        let post n = global<R>(a).v;
        aborts_if !exists<R>(a);
        aborts_if global<R>(a).v + 1 > MAX_U64;
        ensures forall n in 0..n: n < result;
        ensures forall n in n..n + 1: result == n;
        ensures result == old(global<R>(a).v) + 1;
        modifies global<R>(a);
    }

    // Canary: must not verify. At a single state `result == n` with `n` the
    // post-state value contradicts `result == old(v) + 1`.
    fun quant_false(a: address): u64 {
        0
    }
    spec quant_false {
        pragma aborts_if_is_partial;
        requires exists<R>(a);
        requires global<R>(a).v < 100;
        aborts_if result_of<bump>(a) > MAX_U64;
        ensures false;
    }
}
