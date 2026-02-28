//# publish
module 0xc0ffee::m {
    public fun test_match_bool(p: bool): u8 {
        match (p) {
            true => 1,
            false => 0,
        }
    }

    public fun test_match_u8(p: u8): u8 {
        match (p) {
            0 => 0,
            1 => 1,
            _ => 255,
        }
    }

    public fun test_match_u64(p: u64): u64 {
        match (p) {
            0 => 0,
            1 => 1,
            _ => 255,
        }
    }


    public fun test_match_int_tuple(p: u8, q: u8): u8 {
        match ((p, q)) {
            (0, 0) => 0,
            (0, 1) => 1,
            (1, 0) => 2,
            (1, 1) => 3,
            _ => 255,
        }
    }

    public fun test_match_arms(p: u64, q: u64): u64 {
        match (p) {
            0 if q < 100 => 0,
            _ => 5
        }
    }
}

//# run 0xc0ffee::m::test_match_bool --args true

//# run 0xc0ffee::m::test_match_bool --args false

//# run 0xc0ffee::m::test_match_u8 --args 0u8

//# run 0xc0ffee::m::test_match_u8 --args 1u8

//# run 0xc0ffee::m::test_match_u8 --args 42u8

//# run 0xc0ffee::m::test_match_u64 --args 0

//# run 0xc0ffee::m::test_match_u64 --args 1

//# run 0xc0ffee::m::test_match_u64 --args 42

//# run 0xc0ffee::m::test_match_int_tuple --args 0u8 0u8

//# run 0xc0ffee::m::test_match_int_tuple --args 16u8 55u8

//# run 0xc0ffee::m::test_match_arms --args 0 42
