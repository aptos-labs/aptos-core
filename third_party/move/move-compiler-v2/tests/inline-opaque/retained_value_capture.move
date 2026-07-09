// In verify mode, the inline function `apply` is retained as an opaque callee:
// its body is replaced by a stub, the call survives, and the lambda argument is
// lifted into a function carrying its spec block. In normal compilation mode,
// the call is expanded as usual and the spec is dropped.
module 0x42::retained_value_capture {

    inline fun apply(f: |u64| u64, x: u64): u64 {
        f(x)
    }
    spec apply {
        pragma opaque;
        ensures ensures_of<f>(x, result);
    }

    fun caller(x: u64, c: u64): u64 {
        apply(|y| y + c spec { ensures result == y + c; }, x)
    }
    spec caller {
        ensures result == x + c;
    }
}
