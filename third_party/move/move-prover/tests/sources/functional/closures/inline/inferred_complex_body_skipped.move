// flag: --infer-lambda-specs
//
// When the lambda body is too complex for the WP analyzer to summarize
// cleanly, the lambda's spec is left empty (silent skip) and behavioral
// predicates over it degrade to trivial values. The caller's strong post
// fails — status quo for un-summarizable bodies.
module 0x42::inferred_complex_body_skipped {

    inline fun apply(f: |u64| u64, x: u64): u64 {
        f(x)
    }
    spec apply {
        pragma opaque;
        ensures ensures_of<f>(x, result);
    }

    /// Lambda body uses a loop. Backward WP over loops with unknown bounds
    /// cannot summarize the body, so inference silently leaves the spec
    /// empty and the caller's strong claim is not provable.
    fun test(x: u64): u64 {
        apply(|y| {
            let i = 0;
            let s = y;
            while (i < y) {
                s = s + 1;
                i = i + 1;
            };
            s
        }, x)
    }
    spec test {
        requires x < 100;
        ensures result == 2 * x; // error: post-condition does not hold (silent skip; bp_ensures = true)
    }
}
