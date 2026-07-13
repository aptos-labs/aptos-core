// Copyright © Aptos Foundation
// A two-state behavioral predicate over a function value inside a spec
// function body has no pre-state available: unlike concrete closure targets,
// whose old memory is folded into the enclosing spec function's signature,
// the evaluator's memory union is only known per instantiation. Such
// predicates are rejected. (Previously both state slots silently resolved to
// the current state, evaluating the target contract against the wrong state
// pair — e.g. `x == old(x) + 1` became `x == x + 1`.)

module 0x42::spec_fun_bp_fun_value_two_state {

    struct R has key { x: u64 }

    fun bump(a: address) acquires R {
        R[a].x += 1;
    }
    spec bump {
        pragma opaque;
        modifies R[a];
        aborts_if !exists<R>(a) || R[a].x == MAX_U64;
        ensures R[a].x == old(R[a].x) + 1;
    }

    spec fun bumps(h: |address| has drop, a: address): bool {
        ensures_of<h>(a) // error: two-state behavioral predicate over a function value in spec function body
    }

    fun apply(h: |address| has drop, a: address) {
        h(a)
    }
    spec apply {
        requires requires_of<h>(a);
        aborts_if aborts_of<h>(a);
        ensures bumps(h, a);
    }

    // Forces `bump` into the evaluator memory union for `|address|`.
    fun user(a: address) acquires R {
        apply(|x| bump(x), a)
    }
    spec user {
        requires exists<R>(a) && R[a].x < MAX_U64;
    }
}
