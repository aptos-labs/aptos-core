// Copyright © Aptos Foundation

// flag: --check-inconsistency
module 0x42::behavioral_predicate_no_callee_spec {

    fun callee(x: u64): u64 {
        x - 1
    }

    // `callee` has no specification of its own, so `aborts_of<callee>` and
    // `result_of<callee>` are the only description of it available here. The
    // resulting assumptions must stay satisfiable: a caller whose normal exit
    // becomes unreachable proves every postcondition vacuously.
    // error: `callee` publishes no contract, so the behavioral predicates over
    // it are rejected. Before that, `aborts_of<callee>` was defined as `false`,
    // which left this caller's normal exit unreachable and proved every
    // postcondition vacuously.
    fun caller_without_callee_spec(x: u64): u64 {
        callee(x)
    }
    spec caller_without_callee_spec {
        pragma opaque;
        aborts_if aborts_of<callee>(x);
        ensures result == result_of<callee>(x);
    }

    // The same contract over a callee that does carry a specification. This
    // one is consistent, which isolates the difference to the missing callee
    // specification rather than to the behavioral predicates themselves.
    fun specified_callee(x: u64): u64 {
        x - 1
    }
    spec specified_callee {
        pragma opaque;
        aborts_if x == 0;
        ensures result == x - 1;
    }

    fun caller_with_callee_spec(x: u64): u64 {
        specified_callee(x)
    }
    spec caller_with_callee_spec {
        pragma opaque;
        aborts_if aborts_of<specified_callee>(x);
        ensures result == result_of<specified_callee>(x);
    }
}
