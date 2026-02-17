//# publish
module 0xc0ffee::m {
    // Nested match expressions
    public fun test_nested_match(a: bool, b: u8): u8 {
        match (a) {
            true => match (b) {
                0 => 10,
                1 => 11,
                _ => 19,
            },
            false => match (b) {
                0 => 20,
                1 => 21,
                _ => 29,
            },
        }
    }

    // Match in both branches
    public fun test_match_in_branches(x: u8, y: u8): u8 {
        match (x) {
            0 => match (y) {
                0 => 0,
                _ => 1,
            },
            1 => match (y) {
                0 => 2,
                _ => 3,
            },
            _ => 99,
        }
    }
}

//# run 0xc0ffee::m::test_nested_match --args true 0u8

//# run 0xc0ffee::m::test_nested_match --args true 1u8

//# run 0xc0ffee::m::test_nested_match --args true 5u8

//# run 0xc0ffee::m::test_nested_match --args false 0u8

//# run 0xc0ffee::m::test_nested_match --args false 1u8

//# run 0xc0ffee::m::test_nested_match --args false 5u8

//# run 0xc0ffee::m::test_match_in_branches --args 0u8 0u8

//# run 0xc0ffee::m::test_match_in_branches --args 0u8 5u8

//# run 0xc0ffee::m::test_match_in_branches --args 1u8 0u8

//# run 0xc0ffee::m::test_match_in_branches --args 1u8 5u8

//# run 0xc0ffee::m::test_match_in_branches --args 5u8 0u8
