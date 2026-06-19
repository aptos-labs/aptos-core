// In verify mode, lambdas passed to retained inline-opaque functions may capture
// immutable references (the bytecode verifier is skipped since the code is only
// translated for the prover). In normal compilation mode this is an error.
module 0x42::retained_ref_capture {

    inline fun apply(f: |u64| u64, x: u64): u64 {
        f(x)
    }
    spec apply {
        pragma opaque;
        ensures ensures_of<f>(x, result);
    }

    fun caller(r: &u64, x: u64): u64 {
        apply(|y| y + *r spec { ensures result == y + r; }, x)
    }
    spec caller {
        ensures result == x + r;
    }
}
