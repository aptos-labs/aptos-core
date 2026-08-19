// The value spliced for `result_of<f>(..)` from a lambda's attached
// functional `ensures result == E` is subject to the same state resolution
// policy as a value derived from the lambda's body: when `E` references
// global state of the lambda's own two-state scope (e.g. `old(..)` over a
// resource), it cannot be anchored in a value position and must be
// rejected outside of a loop invariant. Without the check, the spliced
// `old(..)` resolves to function entry — the wrong state when the inline
// body changes the state before applying the lambda — which lets a false
// claim verify. (See `result_of_attached_state_ok.move` for the accepted
// state-free and loop-invariant forms.)
module 0x42::result_of_attached_state {

    struct R has key { v: u64 }

    inline fun bump_then_apply(f: |u64| u64, a: address, x: u64): u64 {
        let r = &mut R[a];
        r.v = r.v + 1;
        let res = f(x);
        spec {
            // Semantically false: it claims `res == f's result + 1`. If the
            // attached `old(R[a].v)` were spliced unchecked, it would
            // resolve to function entry — before the bump — making the
            // claim `res == entry.v + x + 1`, which is true: the wrong
            // state turns a false claim into a provable one.
            assert res == result_of<f>(x) + 1; // error: the lambda's result depends on global state
        };
        res
    }

    fun attached_state_result(a: address, x: u64): u64 {
        bump_then_apply(|y| R[a].v + y spec { ensures result == old(R[a].v) + y; }, a, x)
    }
    spec attached_state_result {
        pragma aborts_if_is_partial;
    }

    // Bare (un-`old`-ed) memory reads in the value are rejected the same
    // way: they describe the application's post-state, so splicing them
    // into an assertion evaluated after the inline body changed the state
    // again would also let a false claim verify (here: `res + 1 ==
    // R[a].v + x` holds at the assertion site after the bump, though f's
    // result is `res`, not `res + 1`).
    inline fun apply_then_bump(f: |u64| u64, a: address, x: u64): u64 {
        let res = f(x);
        R[a].v = R[a].v + 1;
        spec {
            assert res + 1 == result_of<f>(x); // error: the lambda's result depends on global state
        };
        res
    }

    // The state-reading value from an attached functional ensures.
    fun attached_bare_read(a: address, x: u64): u64 {
        apply_then_bump(|y| R[a].v + y spec { ensures result == R[a].v + y; }, a, x)
    }
    spec attached_bare_read {
        pragma aborts_if_is_partial;
    }

    // The same value derived from the (spec-less) lambda's body.
    fun derived_bare_read(a: address, x: u64): u64 {
        apply_then_bump(|y| R[a].v + y, a, x)
    }
    spec derived_bare_read {
        pragma aborts_if_is_partial;
    }

    fun read_r(a: address): u64 acquires R {
        borrow_global<R>(a).v
    }

    // The state read can also be hidden behind a default-range behavioral
    // predicate in the attached functional ensures. Evaluating that nested
    // `result_of` after the bump would see the later memory and make the
    // false assertion in `apply_then_bump` verify.
    fun attached_nested_behavior(a: address, x: u64): u64 {
        apply_then_bump(
            |y| read_r(a) + y spec { ensures result == result_of<read_r>(a) + y; },
            a,
            x,
        )
    }
    spec attached_nested_behavior {
        pragma aborts_if_is_partial;
    }
}
