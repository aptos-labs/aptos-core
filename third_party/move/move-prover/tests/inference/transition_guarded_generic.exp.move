// A generic callee whose contract reads memory only through a bare type
// parameter (`spec_exists_at<T>`) writes that memory at `T = S`, where its
// `result_of` axiom is guarded by a transition witness: the inferred caller
// anchors the carrier by the call's `ensures_of` fact, and drops an abort
// condition naming the carrier at a single state.
module 0x2::object {
    spec module {
        fun spec_exists_at<T: key>(object: address): bool;
    }

    public native fun exists_at<T: key>(object: address): bool;
    spec exists_at {
        pragma opaque;
        aborts_if false;
        ensures result == spec_exists_at<T>(object);
    }
}

module 0x42::transition_guarded_generic {
    use 0x2::object;

    struct S has key { v: u64 }

    fun take<T: key>(a: address): bool acquires S {
        if (exists<S>(a)) {
            let S { v: _ } = move_from<S>(a);
        };
        object::exists_at<T>(a)
    }
    spec take {
        pragma opaque;
        // Verifying a generic function here hits an unrelated prelude defect
        // (`$1.cmp.Ordering` undeclared); the frame states the write.
        pragma verify = false;
        pragma inference = none;
        aborts_if false;
        ensures result == object::spec_exists_at<T>(a);
        modifies global<S>(a);
    }

    fun take_s(a: address): bool acquires S {
        take<S>(a)
    }
    spec take_s(a: address): bool {
        pragma opaque = true;
        modifies S[a];
        ensures [inferred] result == result_of<take<S>>(a);
        ensures [inferred] ensures_of<take<S>>(a, result_of<take<S>>(a));
        aborts_if [inferred] false;
    }


    fun take_s_or_abort(a: address) acquires S {
        if (take<S>(a)) abort 7
    }
    spec take_s_or_abort(a: address) {
        pragma opaque = true, aborts_if_is_partial = true;
        modifies S[a];
        ensures [inferred] ensures_of<take<S>>(a, result_of<take<S>>(a));
    }

}
/*
Inference diagnostics:
warning: WP could not characterize the aborts of `transition_guarded_generic::take_s_or_abort` exactly, so its emitted `aborts_if` clauses are a lower bound and the specification carries `aborts_if_is_partial`. Complete the abort behavior and remove that pragma before relying on the contract. Reasons:
  = an abort condition depended on memory written by a call, or on the result of a call which may write memory its contract observes; neither is stated at the entry state
   ┌─ tests/inference/transition_guarded_generic.move:45:5
   │
45 │ ╭     fun take_s_or_abort(a: address) acquires S {
46 │ │         if (take<S>(a)) abort 7
47 │ │     }
   │ ╰─────^

Verification: Succeeded.
*/
