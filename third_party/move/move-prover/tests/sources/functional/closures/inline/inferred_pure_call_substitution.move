// flag: --infer-lambda-specs
//
// `--infer-lambda-specs` also enables `SPEC_REWRITE_PURE_FUNS`, so a lambda
// body that calls a pure helper inferences cleanly to
// `ensures result == y + helper(c, 1)` rather than referencing
// `result_of<helper>(c, 1)`. The caller's strong post then verifies.
module 0x42::inferred_pure_call_substitution {

    fun helper(a: u64, b: u64): u64 {
        a + b
    }
    spec helper {
        aborts_if a + b > MAX_U64;
        ensures result == a + b;
    }

    inline fun apply(f: |u64| u64, x: u64): u64 {
        f(x)
    }
    spec apply {
        pragma opaque;
        ensures ensures_of<f>(x, result);
    }

    fun test(x: u64, c: u64): u64 {
        apply(|y| y + helper(c, 1), x)
    }
    spec test {
        requires x < (1 << 32) && c < (1 << 32);
        ensures result == x + c + 1;
    }
}
