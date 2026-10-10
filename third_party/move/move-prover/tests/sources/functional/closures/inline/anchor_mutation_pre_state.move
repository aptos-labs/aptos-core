// Anchored memory MUTATION ops bind the anchor state, not function entry:
// the derived spec of a state-mutating lambda contains whole-memory effect
// operations (`update<R>(..)` etc.) whose unlabeled pre-state range must
// resolve at the unique application's anchor under `WithStateAnchor`, like
// `old(Global)` reads. Bound to function entry instead, the effect
// operation would compare the post-memory against the wrong base once the
// inline body mutated the same resource — at ANY address — before the
// application, wrongly rejecting a valid `ensures_of` (completeness).
module 0x42::anchor_mutation_pre_state {

    struct R has key { v: u64 }

    inline fun bump_other_then_apply(f: |u64| u64, a: address, b: address, x: u64): u64 {
        R[b].v = R[b].v + 1; // mutation of the same resource before the application
        let res = f(x);
        spec {
            assert ensures_of<f>(x, res); // error: false for the `wrong_attached_claim` caller
        };
        res
    }

    // VALID claim (derived from the body): the lambda updates `a` by 2.
    // The derived update op relates the anchor memory to the post memory;
    // an entry-bound pre-memory would miss the earlier bump at `b`.
    fun valid_derived_claim(a: address, b: address, x: u64): u64 {
        bump_other_then_apply(|y| { R[a].v = R[a].v + 2; y }, a, b, x)
    }
    spec valid_derived_claim {
        requires a != b;
        requires exists<R>(a) && exists<R>(b);
        requires R[a].v < 1000 && R[b].v < 1000;
        pragma aborts_if_is_partial;
    }

    // Soundness canary: a wrong attached claim about the application must
    // still fail under the anchor-bound resolution.
    fun wrong_attached_claim(a: address, b: address, x: u64): u64 {
        bump_other_then_apply(|y| { R[a].v = R[a].v + 2; y }
            spec { ensures R[a].v == old(R[a].v) + 3; ensures result == y; }, a, b, x)
    }
    spec wrong_attached_claim {
        requires a != b;
        requires exists<R>(a) && exists<R>(b);
        requires R[a].v < 1000 && R[b].v < 1000;
        pragma aborts_if_is_partial;
    }
}
