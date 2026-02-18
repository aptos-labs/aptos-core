//# publish
module 0xc0ffee::tuple_literal_match {
    fun match_tuple(x: u8): u8 {
        match ((x, 42)) {
            (v, 42) => v,
            _ => 0,
        }
    }

    public fun test_match_7(): u8 {
        match_tuple(7)
    }

    public fun test_match_0(): u8 {
        match_tuple(0)
    }
}

//# run 0xc0ffee::tuple_literal_match::test_match_7

//# run 0xc0ffee::tuple_literal_match::test_match_0
