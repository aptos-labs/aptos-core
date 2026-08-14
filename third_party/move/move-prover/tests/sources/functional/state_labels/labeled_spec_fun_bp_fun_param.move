// Copyright © Aptos Foundation
// A state label cannot be applied to a spec-function call whose body
// (transitively) contains a behavioral predicate over a function-typed value:
// the predicate's memory footprint is only known per instantiation, so it
// cannot be threaded through the labeled call's memory parameters. Each
// labeled call below is rejected.
module 0x42::labeled_spec_fun_bp_fun_param {
    struct Counter has key { value: u64 }

    spec fun param_wont_abort(f: |address| has copy + drop, a: address): bool {
        !aborts_of<f>(a)
    }

    spec fun param_wraps(f: |address| has copy + drop, a: address): bool {
        param_wont_abort(f, a)
    }

    fun apply(f: |address| has copy + drop, a: address) {
        f(a)
    }
    spec apply {
        pragma opaque;
        modifies_of<f>(x: address) Counter[x];
        aborts_if aborts_of<f>(a);
        ensures exists S in *: S |~ param_wont_abort(f, a); // error: BP over a function-typed value
        ensures exists S in *: S |~ param_wraps(f, a); // error: transitively BP over a function-typed value
    }
}
