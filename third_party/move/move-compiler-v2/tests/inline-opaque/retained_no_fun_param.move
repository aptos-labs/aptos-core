// An inline function without function-typed parameters can carry a spec. With
// `pragma opaque` it is retained (not expanded) in verify mode and call sites
// use its spec; in normal compilation it is expanded and the spec is dropped.
module 0x42::retained_no_fun_param {

    inline fun increment(x: u64): u64 {
        x + 1
    }
    spec increment {
        pragma opaque;
        aborts_if x == 18446744073709551615;
        ensures result == x + 1;
    }

    fun caller(x: u64): u64 {
        increment(x)
    }
    spec caller {
        aborts_if x == 18446744073709551615;
        ensures result == x + 1;
    }
}
