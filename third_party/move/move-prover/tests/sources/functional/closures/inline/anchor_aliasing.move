// State-snapshot anchoring when ONE spec condition constrains TWO distinct
// state-effecting lambda parameters: each `ensures_of` conjunct must read
// its `old(..)` pre-state at its own parameter's application site. The
// snapshots must not alias — `ensures_of<f>`'s pre-state is taken before
// `f` runs, `ensures_of<g>`'s before `g` runs, even when both conjuncts
// read the same resource. (With aliased snapshots, f's "pre-state" would
// be taken at g's application site — after f ran — which both lets a false
// two-state claim about f verify and makes a true one fail.)
module 0x42::anchor_aliasing {

    struct R has key { v: u64 }
    struct T has key { v: u64 }

    inline fun apply_two(f: |address|, g: |address|, a: address) {
        f(a);
        g(a);
        spec {
            // ONE condition over both parameters: f's conjunct is anchored
            // at f's application site, g's at g's.
            assert ensures_of<f>(a) && ensures_of<g>(a); // error: for the `bump_twice*` callers, f's conjunct is false: from f's pre-state to the assertion state the value moved by +2, not +1
        };
    }

    // ===== NEGATIVE canary: both lambdas bump the SAME R[a].v by 1 =====
    // f's conjunct claims `R[a].v == old(R[a].v) + 1` between f's pre-state
    // and the assertion state — the actual difference is +2, so the assert
    // in `apply_two` must fail.

    // With lambda specs attached: the substituted conditions are pure
    // value claims over the anchored snapshots.
    fun bump_twice_spec(a: address) {
        apply_two(
            |x| { let r = &mut R[x]; r.v = r.v + 1; } spec { ensures R[x].v == old(R[x].v) + 1; },
            |x| { let r = &mut R[x]; r.v = r.v + 1; } spec { ensures R[x].v == old(R[x].v) + 1; },
            a,
        );
    }
    spec bump_twice_spec {
        requires exists<R>(a) && R[a].v < MAX_U64 - 1;
        ensures R[a].v == old(R[a].v) + 2;
    }

    // With spec-less lambdas: the derived whole-memory update effects read
    // their old-state values through the anchored snapshots.
    fun bump_twice_derived(a: address) {
        apply_two(
            |x| { let r = &mut R[x]; r.v = r.v + 1; },
            |x| { let r = &mut R[x]; r.v = r.v + 1; },
            a,
        );
    }
    spec bump_twice_derived {
        requires exists<R>(a) && R[a].v < MAX_U64 - 1;
        ensures R[a].v == old(R[a].v) + 2;
    }

    // ===== POSITIVE: f and g write DISJOINT resources =====
    // Both conjuncts are true under correct anchoring: g does not touch R,
    // so f's ensures also holds from f's pre-state to the assertion point.

    fun bump_disjoint_spec(a: address) {
        apply_two(
            |x| { let r = &mut R[x]; r.v = r.v + 1; } spec { ensures R[x].v == old(R[x].v) + 1; },
            |x| { let t = &mut T[x]; t.v = t.v + 1; } spec { ensures T[x].v == old(T[x].v) + 1; },
            a,
        );
    }
    spec bump_disjoint_spec {
        requires exists<R>(a) && R[a].v < MAX_U64;
        requires exists<T>(a) && T[a].v < MAX_U64;
        ensures R[a].v == old(R[a].v) + 1;
        ensures T[a].v == old(T[a].v) + 1;
    }

    fun bump_disjoint_derived(a: address) {
        apply_two(
            |x| { let r = &mut R[x]; r.v = r.v + 1; },
            |x| { let t = &mut T[x]; t.v = t.v + 1; },
            a,
        );
    }
    spec bump_disjoint_derived {
        requires exists<R>(a) && R[a].v < MAX_U64;
        requires exists<T>(a) && T[a].v < MAX_U64;
        ensures R[a].v == old(R[a].v) + 1;
        ensures T[a].v == old(T[a].v) + 1;
    }
}
