// Negative tests for the verification of inline function bodies against their
// specs: a body violating the spec is reported at the inline function, and
// `pragma verify = false` opts out.
module 0x42::opaque_inline_body_fail {

    inline fun inc(x: u64): u64 {
        x + 2
    }
    spec inc {
        pragma opaque;
        aborts_if x + 2 > MAX_U64;
        ensures result == x + 1; // error: post-condition does not hold (body adds 2)
    }

    fun test_inc(x: u64): u64 {
        inc(x)
    }
    spec test_inc {
        aborts_if x + 2 > MAX_U64;
        ensures result == x + 1;
    }

    inline fun inc_trusted(x: u64): u64 {
        x + 2
    }
    spec inc_trusted {
        pragma opaque;
        pragma verify = false;
        aborts_if x + 2 > MAX_U64;
        ensures result == x + 1;
    }

    /// No body verification for `inc_trusted`; callers use its (wrong) spec.
    fun test_inc_trusted(x: u64): u64 {
        inc_trusted(x)
    }
    spec test_inc_trusted {
        aborts_if x + 2 > MAX_U64;
        ensures result == x + 1;
    }
}
