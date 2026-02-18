//# publish
module 0xc0ffee::m {
    // Guard references a variable bound by a tuple pattern.
    // The tuple has a literal at position 0 and a variable at position 1.
    // The guard should be able to see `v`.
    public fun tuple_guard_uses_var(x: u64, y: u64): u64 {
        match ((x, y)) {
            (1, v) if (v > 10) => v + 100,
            (1, v) => v,
            _ => 0,
        }
    }

    // Guard references a variable at position 0, literal at position 1.
    public fun tuple_guard_first_var(x: u64, y: u64): u64 {
        match ((x, y)) {
            (a, 42) if (a > 5) => a * 10,
            (a, 42) => a,
            _ => 0,
        }
    }

    // Both positions are variables, with a guard referencing both.
    public fun tuple_guard_both_vars(x: u64, y: u64): u64 {
        match ((x, y)) {
            (a, b) if (a + b > 10) => a + b,
            _ => 0,
        }
    }
}

//# run 0xc0ffee::m::tuple_guard_uses_var --args 1u64 20u64

//# run 0xc0ffee::m::tuple_guard_uses_var --args 1u64 5u64

//# run 0xc0ffee::m::tuple_guard_uses_var --args 2u64 20u64

//# run 0xc0ffee::m::tuple_guard_first_var --args 10u64 42u64

//# run 0xc0ffee::m::tuple_guard_first_var --args 3u64 42u64

//# run 0xc0ffee::m::tuple_guard_first_var --args 10u64 99u64

//# run 0xc0ffee::m::tuple_guard_both_vars --args 8u64 5u64

//# run 0xc0ffee::m::tuple_guard_both_vars --args 3u64 2u64
