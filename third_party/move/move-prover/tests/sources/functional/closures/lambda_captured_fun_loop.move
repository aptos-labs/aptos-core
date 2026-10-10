// A loop-containing lambda invoking a CAPTURED function value: partial
// `ensures_of` anchors over the non-concrete target go through
// `translate_behavior_via_evaluator`, which pads the missing post-state
// slot from the per-type result-of Skolem. The pinned expectation is
// verification verdicts (loop-lambda inferred contracts are not provable
// against their bodies — a known inference-completeness limit), NOT Boogie
// arity errors: this file regresses to `wrong number of arguments` if the
// padding breaks.
module 0x42::lambda_captured_fun_loop {
    public fun apply_mut(f: |&mut u64| has drop, x: &mut u64) {
        f(x)
    }
    spec apply_mut {
        pragma opaque;
        pragma verify = false;
        requires requires_of<f>(x);
        aborts_if aborts_of<f>(x);
        ensures ensures_of<f>(old(x), x);
    }

    public fun wrap2(g: |&mut u64| has copy + drop, x: &mut u64, n: u64) {
        apply_mut(|v| {
            let i = 0;
            while (i < n) {
                g(v);
                i += 1;
            };
        }, x)
    }
}
