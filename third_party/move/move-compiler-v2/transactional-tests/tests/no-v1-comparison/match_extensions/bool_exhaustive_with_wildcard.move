//# publish
module 0xc0ffee::m {
    // Boolean match with wildcard should be exhaustive
    public fun test_bool_wildcard(p: bool): u8 {
        match (p) {
            true => 1,
            _ => 0,
        }
    }

    // Just wildcard is also exhaustive
    public fun test_only_wildcard(p: bool): u8 {
        match (p) {
            _ => 42,
        }
    }
}

//# run 0xc0ffee::m::test_bool_wildcard --args true

//# run 0xc0ffee::m::test_bool_wildcard --args false

//# run 0xc0ffee::m::test_only_wildcard --args true

//# run 0xc0ffee::m::test_only_wildcard --args false
