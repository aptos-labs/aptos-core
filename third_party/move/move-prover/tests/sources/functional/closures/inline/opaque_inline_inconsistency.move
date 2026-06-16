// flag: --check-inconsistency
//
// Guards against vacuity in the verification context created for retained
// inline-opaque calls: the `ensures_of` assumptions injected for a value
// capture must be satisfiable.
module 0x42::opaque_inline_inconsistency {

    inline fun apply(f: |u64| u64, x: u64): u64 {
        f(x)
    }
    spec apply {
        pragma opaque;
        ensures ensures_of<f>(x, result);
    }

    fun value_capture(x: u64, c: u64): u64 {
        apply(|y| y + c spec { ensures result == y + c; }, x)
    }
    spec value_capture {
        ensures result == x + c;
    }
}
