//# publish
module 0xc0ffee::m {
    // Tuple with mixed literals and wildcards
    public fun test_mixed(a: u8, b: u8): u8 {
        match ((a, b)) {
            (0, _) => 0,
            (_, 0) => 1,
            (_, _) => 2,
        }
    }

    // Simple tuple pattern
    public fun test_simple_tuple(a: u8, b: u8): u8 {
        match ((a, b)) {
            (0, 0) => 0,
            (0, 1) => 1,
            (1, 0) => 2,
            (1, 1) => 3,
            _ => 255,
        }
    }

    // Triple tuple
    public fun test_triple(a: u8, b: u8, c: u8): u8 {
        match ((a, b, c)) {
            (0, 0, 0) => 0,
            (0, 0, _) => 1,
            (0, _, _) => 2,
            (_, _, _) => 3,
        }
    }
}

//# run 0xc0ffee::m::test_mixed --args 0u8 10u8

//# run 0xc0ffee::m::test_mixed --args 10u8 0u8

//# run 0xc0ffee::m::test_mixed --args 5u8 5u8

//# run 0xc0ffee::m::test_simple_tuple --args 0u8 0u8

//# run 0xc0ffee::m::test_simple_tuple --args 0u8 1u8

//# run 0xc0ffee::m::test_simple_tuple --args 1u8 0u8

//# run 0xc0ffee::m::test_simple_tuple --args 1u8 1u8

//# run 0xc0ffee::m::test_simple_tuple --args 5u8 5u8

//# run 0xc0ffee::m::test_triple --args 0u8 0u8 0u8

//# run 0xc0ffee::m::test_triple --args 0u8 0u8 1u8

//# run 0xc0ffee::m::test_triple --args 0u8 1u8 1u8

//# run 0xc0ffee::m::test_triple --args 1u8 1u8 1u8
