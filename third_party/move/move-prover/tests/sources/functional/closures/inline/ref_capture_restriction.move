// In verification, closures capturing references are only supported as direct
// arguments of calls to retained inline-opaque functions, where the captured
// locations are statically visible to the spec instrumentation.
module 0x42::ref_capture_restriction {

    fun regular_apply(f: |u64| u64, x: u64): u64 {
        f(x)
    }

    fun test_ref_capture_to_regular_fun(r: &u64, x: u64): u64 {
        regular_apply(|y| y + *r, x) // error: captured value cannot be a reference
    }

    fun test_mut_capture_to_regular_fun(x: u64): u64 {
        let c = 0;
        regular_apply(|y| {
            c = c + 1; // error: captured value cannot be a reference
            y + c
        }, x)
    }

    struct Wrap has drop {
        f: |u64| has drop,
    }

    /// Test: a reference-capturing closure cannot be hidden inside a struct
    /// value (the field initializer is not a direct call argument).
    fun test_ref_capture_in_struct(): u64 {
        let x = 0;
        let _w = Wrap { f: |i| x = x + i spec { ensures x == old(x) + i; } }; // error: captured value cannot be a reference
        x
    }
}
