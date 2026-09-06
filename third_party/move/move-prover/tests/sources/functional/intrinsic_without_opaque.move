// An unregistered `pragma intrinsic` must not suppress verification of an
// executable function body. Otherwise a false contract would be assumed by
// callers without the body ever being checked.
module 0x42::intrinsic_without_opaque {

    fun bounded(x: u64): u64 {
        if (x > 10) { 10 } else { x }
    }

    spec bounded {
        pragma intrinsic;
        ensures result == x; // error: false when x > 10
    }

    fun caller(x: u64): u64 {
        bounded(x)
    }

    spec caller {
        ensures result == x;
    }

    fun caller_expecting_the_body(x: u64): u64 {
        bounded(x)
    }

    spec caller_expecting_the_body {
        ensures result == x;
    }
}
