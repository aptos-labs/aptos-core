// A lambda whose body loops and calls a `&mut` callee with a discarded
// result: WP's inferred contract carries partial-arity `ensures_of` anchors
// that canonicalization does not cover, so the behavioral evaluator pads the
// missing trailing post-state slots from the callee's ensures-results Skolem
// (`translate_behavior_for_closure`). The pinned expectation is VERIFICATION
// verdicts — the inferred contract of a loop-containing lambda is not
// provable against its body (a known inference-completeness limit) — and
// NOT Boogie name-resolution/type errors: this file regresses to
// `wrong number of arguments` internal errors if the padding breaks.
module 0x42::lambda_spec_loop_anchor {
    use std::vector;

    public fun apply(f: |&mut vector<u64>| has drop, x: &mut vector<u64>) {
        f(x)
    }
    spec apply {
        pragma opaque;
        pragma verify = false;
        requires requires_of<f>(x);
        aborts_if aborts_of<f>(x);
        ensures ensures_of<f>(old(x), x);
    }

    public fun drive(x: &mut vector<u64>) {
        apply(|v: &mut vector<u64>| {
            let i = 0;
            while (i < vector::length(v)) {
                if (*vector::borrow(v, i) == 0) {
                    vector::remove(v, i);
                } else {
                    i += 1;
                };
            };
        }, x)
    }
}
