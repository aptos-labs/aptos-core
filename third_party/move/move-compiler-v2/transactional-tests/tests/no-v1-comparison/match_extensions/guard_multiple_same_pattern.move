//# publish
module 0xc0ffee::m {
    // Multiple guards on same pattern value
    public fun test_multiple_guards(x: u64, a: bool, b: bool): u64 {
        match (x) {
            0 if a => 1,
            0 if b => 2,
            0 => 3,
            _ => 4,
        }
    }

    // Guards with complex conditions
    public fun test_complex_guard(x: u8, threshold: u8): u8 {
        match (x) {
            0 if threshold > 10 => 0,
            0 if threshold > 5 => 1,
            0 => 2,
            _ => 3,
        }
    }
}

//# run 0xc0ffee::m::test_multiple_guards --args 0u64 true false

//# run 0xc0ffee::m::test_multiple_guards --args 0u64 false true

//# run 0xc0ffee::m::test_multiple_guards --args 0u64 false false

//# run 0xc0ffee::m::test_multiple_guards --args 1u64 true true

//# run 0xc0ffee::m::test_complex_guard --args 0u8 15u8

//# run 0xc0ffee::m::test_complex_guard --args 0u8 7u8

//# run 0xc0ffee::m::test_complex_guard --args 0u8 3u8

//# run 0xc0ffee::m::test_complex_guard --args 5u8 10u8
