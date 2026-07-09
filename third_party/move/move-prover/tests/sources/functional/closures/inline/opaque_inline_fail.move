// Negative tests for inline higher-order functions with `pragma opaque`:
// wrong lambda specs and wrong caller assertions must be reported.
module 0x42::opaque_inline_fail {

    inline fun apply(f: |u64| u64, x: u64): u64 {
        f(x)
    }
    spec apply {
        pragma opaque;
        ensures ensures_of<f>(x, result);
    }

    /// Test: caller postcondition does not follow from the lambda's spec.
    fun test_wrong_caller_post(x: u64): u64 {
        apply(|y| y + 1 spec { ensures result == y + 1; }, x)
    }
    spec test_wrong_caller_post {
        ensures result == x + 2; // error: post-condition does not hold
    }

    /// Test: the lambda's spec does not hold for its body.
    fun test_wrong_lambda_spec(x: u64): u64 {
        apply(|y| y + 1 spec { ensures result == y + 2; }, x) // error: post-condition does not hold (lifted lambda)
    }
    spec test_wrong_lambda_spec {
        ensures result == x + 2;
    }
}
