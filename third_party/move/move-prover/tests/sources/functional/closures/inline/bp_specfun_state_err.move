// Rejected forms of behavioral predicates over lambdas whose specs access
// global state through SPEC FUNCTIONS (companion of
// `bp_specfun_state.move` and `two_state_spec_fun.move`):
//
// - a pre-state condition reading memory through a spec function needs the
//   unique-application anchor, exactly like a direct read;
// - a `result_of` value phrased through a memory-reading spec function is
//   state-dependent and cannot be anchored in a value position;
// - a two-state spec function (body uses `old(..)`) cannot appear in
//   `requires`/`aborts_if` conditions, which describe a single state.
module 0x42::bp_specfun_state_err {

    struct R has key { v: u64 }

    spec fun spec_get_v(a: address): u64 {
        global<R>(a).v
    }

    // Two-state: relates the current state to the pre-state.
    spec fun spec_delta(a: address): u64 {
        global<R>(a).v - old(global<R>(a).v)
    }

    // ===== no unique application site to anchor the pre-state =====

    inline fun apply_twice(f: |u64| u64, a: address, x: u64): u64 {
        let r1 = f(x);
        R[a].v = R[a].v + 1;
        let r2 = f(r1);
        spec {
            assert requires_of<f>(x); // error: needs a unique application of the function parameter
        };
        r2
    }

    fun no_anchor(a: address, x: u64): u64 {
        apply_twice(|y| y spec { requires spec_get_v(a) > 0; ensures result == y; }, a, x)
    }
    spec no_anchor {
        requires exists<R>(a) && R[a].v == 0;
        pragma aborts_if_is_partial;
    }

    // ===== state-dependent result_of value =====

    inline fun apply_then_bump(f: |u64| u64, a: address, x: u64): u64 {
        let res = f(x);
        R[a].v = R[a].v + 1;
        spec {
            assert res == result_of<f>(x); // error: result depends on global state (through the spec function)
        };
        res
    }

    fun state_dependent_result(a: address, x: u64): u64 {
        apply_then_bump(|y| R[a].v + y spec { ensures result == spec_get_v(a) + y; }, a, x)
    }
    spec state_dependent_result {
        requires exists<R>(a) && R[a].v < 1000;
        requires x < 1000;
        pragma aborts_if_is_partial;
    }

    // ===== two-state spec function in single-state conditions =====

    inline fun apply_once(f: |u64| u64, a: address, x: u64): u64 {
        let res = f(x);
        spec {
            assert requires_of<f>(x); // error: two-state spec function in a `requires` condition
        };
        res
    }

    fun two_state_requires(a: address, x: u64): u64 {
        apply_once(|y| y spec { requires spec_delta(a) == 0; ensures result == y; }, a, x)
    }
    spec two_state_requires {
        requires exists<R>(a);
        pragma aborts_if_is_partial;
    }

    inline fun apply_once_na(f: |u64| u64, a: address, x: u64): u64 {
        let res = f(x);
        spec {
            assert !aborts_of<f>(x); // error: two-state spec function in an `aborts_if` condition
        };
        res
    }

    fun two_state_aborts(a: address, x: u64): u64 {
        apply_once_na(|y| y spec { aborts_if spec_delta(a) > 0; ensures result == y; }, a, x)
    }
    spec two_state_aborts {
        requires exists<R>(a);
        pragma aborts_if_is_partial;
    }
}
