//# publish
module 0xc0ffee::m {
    // Test guarded wildcard/variable arms in match expressions.
    // These exercise the generate_pattern_condition path for Var/Wildcard patterns.

    public fun guarded_var(x: u64): u64 {
        match (x) {
            1 => 10,
            y if (y > 5) => y + 20,
            _ => 30,
        }
    }

    public fun guarded_wildcard(x: u64): u64 {
        match (x) {
            0 => 100,
            _ if (x < 10) => 200,
            _ => 300,
        }
    }
}

//# run 0xc0ffee::m::guarded_var --args 1u64

//# run 0xc0ffee::m::guarded_var --args 10u64

//# run 0xc0ffee::m::guarded_var --args 3u64

//# run 0xc0ffee::m::guarded_wildcard --args 0u64

//# run 0xc0ffee::m::guarded_wildcard --args 5u64

//# run 0xc0ffee::m::guarded_wildcard --args 50u64
