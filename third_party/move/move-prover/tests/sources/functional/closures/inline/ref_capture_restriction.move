// In verification, closures capturing immutable references are only supported as
// direct arguments of calls to retained inline-opaque functions, where the captured
// locations are statically visible to the spec instrumentation. A reference capture
// for any other call, or one hidden inside a struct value, is rejected by the
// closure checker. (Modifying a captured variable is a separate error reported at
// lambda lifting; see the compiler-v2 `inline-opaque` tests.)
module 0x42::ref_capture_restriction {

    fun regular_apply(f: |u64| u64, x: u64): u64 {
        f(x)
    }

    /// Test: a reference capture is not admitted for a regular (non inline-opaque)
    /// call.
    fun test_ref_capture_to_regular_fun(r: &u64, x: u64): u64 {
        regular_apply(|y| y + *r, x) // error: captured value cannot be a reference
    }

    struct Wrap has drop {
        f: |u64| u64 has drop,
    }

    /// Test: a reference-capturing closure cannot be hidden inside a struct value
    /// (the field initializer is not a direct call argument).
    fun test_ref_capture_in_struct(r: &u64): u64 {
        let _w = Wrap { f: |y| y + *r }; // error: captured value cannot be a reference
        0
    }
}
