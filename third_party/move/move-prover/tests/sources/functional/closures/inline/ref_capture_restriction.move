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

    inline fun call_twice(f: |u64| has copy) {
        f(1);
        f(2)
    }
    spec call_twice {
        pragma opaque;
        ensures ensures_of<f>(1);
        ensures ensures_of<f>(2);
    }

    /// Test: a closure capturing references must not be copyable; otherwise the
    /// callee could apply it more than once, while the prover models the effect
    /// on the captured locations as exactly one application.
    fun test_copyable_mut_capture(): u64 {
        let x = 0;
        call_twice(|i| x = x + i spec { ensures x == old(x) + i; }); // error: must not have the `copy` ability
        x
    }
}
