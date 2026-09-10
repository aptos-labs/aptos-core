// The `SaveStateAnchor` marker of a unique application site must snapshot
// the state AFTER the invocation's arguments are evaluated: the lambda's
// pre-state is the state in which the lambda starts executing. When an
// argument expression itself mutates global state, a snapshot taken before
// argument evaluation would attribute the argument's effect to the lambda —
// accepting a false two-state claim about the lambda and rejecting a true
// one.
module 0x42::anchor_arg_effects {

    struct R has key { v: u64 }

    fun bump(a: address): address acquires R {
        let r = &mut R[a];
        r.v = r.v + 1;
        a
    }
    spec bump {
        aborts_if !exists<R>(a) || R[a].v + 1 > MAX_U64;
        ensures R[a].v == old(R[a].v) + 1;
        ensures result == a;
    }

    // The argument expression `bump(a)` mutates R[a] before `f` runs.
    inline fun apply_to_bumped(f: |address|, a: address) {
        f(bump(a));
        spec {
            assert ensures_of<f>(a); // error: for the `noop_after_bump` caller, the lambda changes nothing, so its claimed +1 effect is false
        };
    }

    // POSITIVE: the lambda's ensures is true about the lambda itself: +1
    // from ITS pre-state, which already includes the argument's bump.
    fun bump_after_bump(a: address) {
        apply_to_bumped(
            |x| { let r = &mut R[x]; r.v = r.v + 1; } spec { ensures R[x].v == old(R[x].v) + 1; },
            a,
        );
    }
    spec bump_after_bump {
        requires exists<R>(a) && R[a].v < MAX_U64 - 1;
        ensures R[a].v == old(R[a].v) + 2;
    }

    // NEGATIVE canary: the lambda does nothing, so its claimed +1 effect is
    // false; the assert in `apply_to_bumped` must fail. A pre-argument
    // snapshot would account the argument's bump to the lambda and let this
    // verify.
    fun noop_after_bump(a: address) {
        apply_to_bumped(
            |x| { let _ = x; } spec { ensures R[x].v == old(R[x].v) + 1; },
            a,
        );
    }
    spec noop_after_bump {
        requires exists<R>(a) && R[a].v < MAX_U64 - 1;
        ensures R[a].v == old(R[a].v) + 1;
    }
}
