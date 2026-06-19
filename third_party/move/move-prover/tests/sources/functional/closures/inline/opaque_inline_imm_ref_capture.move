// Tests for lambdas capturing immutable references, passed to retained
// inline-opaque functions. Reference captures are admitted in verify mode
// only (the VM rejects them), since the bytecode is never executed.
module 0x42::opaque_inline_imm_ref_capture {
    use std::vector;

    inline fun apply(f: |u64| u64, x: u64): u64 {
        f(x)
    }
    spec apply {
        pragma opaque;
        ensures ensures_of<f>(x, result);
    }

    /// Test: lambda capturing an immutable reference parameter.
    fun test_ref_param_capture(r: &u64, x: u64): u64 {
        apply(|y| y + *r spec { ensures result == y + r; }, x)
    }
    spec test_ref_param_capture {
        ensures result == x + r;
    }

    /// Test: lambda capturing a locally borrowed vector.
    fun test_local_borrow_capture(x: u64): u64 {
        let v = vector[1, 2, 3];
        let r = &v;
        apply(|y| y + vector::length(r) spec { ensures result == y + len(r); }, x)
    }
    spec test_local_borrow_capture {
        ensures result == x + 3;
    }
}
