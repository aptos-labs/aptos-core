// Copyright © Aptos Foundation

// flag: --check-inconsistency
// flag: --unconditional-abort-as-inconsistency
// The Verex lowering rejects a contract binding sharing a name with a
// quantifier binder (VEREX-CONTRACT-BINDING-DUPLICATE on `S`).
// exclude_for: lean

// A state-domain binder (`exists S in *`) names a state, not a local: a spec
// `let S` referenced from the quantifier body is still that `let`, and the
// clause is judged with the binding in scope. Dropping it leaves `S` unbound
// in the behavioral predicates of the function.
module 0x42::result_of_transition_state_label_let {
    struct R has key { v: u64 }

    // Pre-state `let` sharing its name with the state label.
    fun bump_pre(a: address): u64 acquires R {
        let r = borrow_global_mut<R>(a);
        r.v = r.v + 1;
        r.v
    }
    spec bump_pre {
        pragma opaque;
        let S = global<R>(a).v;
        aborts_if !exists<R>(a);
        aborts_if S + 1 > MAX_U64;
        ensures exists S in *: (S |~ exists<R>(a)) && result == S + 1;
        modifies global<R>(a);
    }

    // The `let` reaches the carrier: between two equal states, the clause
    // ties the result to the value of `v`.
    fun pre_holds(_a: address) {}
    spec pre_holds {
        requires exists<R>(_a);
        requires global<R>(_a).v < 100;
        ensures forall r: u64: ensures_of<bump_pre>(_a, r) ==> r == global<R>(_a).v + 1;
    }

    // Canary: must not verify (the carrier is satisfiable).
    fun pre_holds_false(_a: address) {}
    spec pre_holds_false {
        requires exists<R>(_a);
        requires global<R>(_a).v < 100;
        ensures forall r: u64: !ensures_of<bump_pre>(_a, r);
    }

    // Canary: must not verify.
    fun pre_false(a: address): u64 {
        0
    }
    spec pre_false {
        pragma aborts_if_is_partial;
        requires exists<R>(a);
        requires global<R>(a).v < 100;
        aborts_if result_of<bump_pre>(a) > MAX_U64;
        ensures false;
    }

    // Post-state `let` sharing its name with the state label; the clause
    // reads the post-state through `S` and stays out of the unguarded
    // pre-state relation.
    fun bump_post(a: address): u64 acquires R {
        let r = borrow_global_mut<R>(a);
        r.v = r.v + 1;
        r.v
    }
    spec bump_post {
        pragma opaque;
        let post S = global<R>(a).v;
        aborts_if !exists<R>(a);
        aborts_if global<R>(a).v + 1 > MAX_U64;
        ensures exists S in *: (S |~ exists<R>(a)) && result == S;
        ensures result == old(global<R>(a).v) + 1;
        modifies global<R>(a);
    }

    // Canary: must not verify. At a single state `result == S` with `S` the
    // post-state value contradicts `result == old(v) + 1`.
    fun post_false(a: address): u64 {
        0
    }
    spec post_false {
        pragma aborts_if_is_partial;
        requires exists<R>(a);
        requires global<R>(a).v < 100;
        aborts_if result_of<bump_post>(a) > MAX_U64;
        ensures false;
    }
}
