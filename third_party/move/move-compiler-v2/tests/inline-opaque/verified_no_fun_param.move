// An inline function without function-typed parameters and a non-opaque spec:
// in verify mode the body is compiled and verified standalone, while call sites
// expand the body as usual.
module 0x42::verified_no_fun_param {

    inline fun add(x: u64, y: u64): u64 {
        x + y
    }
    spec add {
        ensures result == x + y;
    }

    fun caller(x: u64): u64 {
        add(x, 1)
    }
}
