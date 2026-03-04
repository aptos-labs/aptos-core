//# publish
module 0xc0ffee::m {
    // Test tuple patterns where some positions are variables (not just literals or wildcards).

    public fun tuple_var_binding(x: u64, y: u64): u64 {
        match ((x, y)) {
            (1, v) => v + 100,
            (2, v) => v + 200,
            (_, v) => v,
        }
    }

    public fun tuple_mixed_var(x: u64, y: u64): u64 {
        match ((x, y)) {
            (a, 1) => a * 10,
            (a, _) => a,
        }
    }
}

//# run 0xc0ffee::m::tuple_var_binding --args 1u64 42u64

//# run 0xc0ffee::m::tuple_var_binding --args 2u64 7u64

//# run 0xc0ffee::m::tuple_var_binding --args 99u64 5u64

//# run 0xc0ffee::m::tuple_mixed_var --args 3u64 1u64

//# run 0xc0ffee::m::tuple_mixed_var --args 3u64 99u64
