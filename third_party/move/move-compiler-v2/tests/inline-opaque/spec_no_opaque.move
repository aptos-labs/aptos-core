// An inline function with a non-opaque spec block: in verify mode the body is
// compiled and verified standalone, while call sites expand the body as usual.
module 0x42::spec_no_opaque {

    inline fun apply(f: |u64| u64, x: u64): u64 {
        f(x)
    }
    spec apply {
        ensures ensures_of<f>(x, result);
    }

    fun caller(x: u64): u64 {
        apply(|y| y + 1, x)
    }
}
