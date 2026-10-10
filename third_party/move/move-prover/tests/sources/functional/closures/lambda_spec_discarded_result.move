// Regression test for inferred lambda contracts over calls whose result is
// DISCARDED: WP leaves a "discarded-result anchor" — `ensures_of` carrying
// inputs and synthesized `result_of` projections but no trailing post-state
// slots. The canonicalizer must replace it with the full-arity form (the
// behavioral evaluator encoding expects the post-state arguments), else the
// emitted evaluator body calls the callee's `$bp_ensures_of` one argument
// short. The guard exercises the wrapped-anchor path.
module 0x42::lambda_spec_discarded_result {
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
        apply(|v| {
            vector::reverse(v);
            if (vector::length(v) > 0) {
                vector::remove(v, 0); // result discarded
            };
        }, x)
    }
}
