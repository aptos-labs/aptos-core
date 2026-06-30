// In verify mode, a lambda passed to a retained inline-opaque function may capture
// only immutable references; capturing a `&mut` is rejected by the closure checker,
// since the prover cannot model writes through a captured mutable reference. In
// normal compilation the inline function is expanded, so the lambda is never lifted
// into a closure and no capture arises.
module 0x42::retained_mut_ref_capture {

    inline fun apply(f: |u64| u64, x: u64): u64 {
        f(x)
    }
    spec apply {
        pragma opaque;
        ensures ensures_of<f>(x, result);
    }

    fun direct_mut_capture(r: &mut u64): u64 {
        apply(|y| y + *r, 5)
    }
}
