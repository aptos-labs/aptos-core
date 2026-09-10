module 0x42::behavioral_soundness {
    use std::type_name;

    fun always_aborts(_x: u64): u64 {
        abort 1
    }
    spec always_aborts {
        aborts_if true;
        ensures false;
    }

    fun always_aborts_context_is_consistent() {}
    spec always_aborts_context_is_consistent {
        ensures result_of<always_aborts>(0) >= 0;
        // error: the result axiom is vacuous when the closure always aborts
        ensures false;
    }

    fun partially_specified_abort(x: u64) {
        assert!(x != 0 && x != 1, 1);
    }
    spec partially_specified_abort {
        pragma aborts_if_is_partial;
        aborts_if x == 0;
    }

    fun partial_abort_predicate_is_not_complete(x: u64) { let _ = x; }
    spec partial_abort_predicate_is_not_complete {
        requires x != 0;
        // error: omitted abort cases leave the predicate unconstrained
        ensures !aborts_of<partially_specified_abort>(x);
    }

    fun requires_positive(x: u64): u64 { x }
    spec requires_positive {
        requires x > 0;
        ensures result > 0;
    }

    fun result_of_outside_precondition_is_consistent() {}
    spec result_of_outside_precondition_is_consistent {
        ensures result_of<requires_positive>(0) >= 0;
        // error: result_of outside a precondition does not imply ensures_of
        ensures false;
    }

    fun generic_reflection_identity<T>(x: u64): u64 {
        x
    }
    spec generic_reflection_identity {
        ensures type_name::get<T>() == type_name::get<T>();
        ensures result == x;
    }

    fun generic_reflection_ensures_are_not_assumed<T>() {}
    spec generic_reflection_ensures_are_not_assumed {
        // error: generic behavioral ensures remain uninterpreted
        ensures ensures_of<generic_reflection_identity<T>>(0, 1);
    }

    fun generic_requires_positive<T>(x: u64): u64 {
        x
    }
    spec generic_requires_positive {
        requires x > 0;
        ensures result == x;
    }

    fun generic_result_outside_precondition_is_not_fixed<T>() {}
    spec generic_result_outside_precondition_is_not_fixed {
        // error: generic result_of is unconstrained outside its precondition
        ensures result_of<generic_requires_positive<T>>(0) == 0;
    }

}
