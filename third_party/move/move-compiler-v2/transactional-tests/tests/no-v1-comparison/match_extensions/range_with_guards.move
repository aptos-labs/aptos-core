//# publish
module 0xc0ffee::m {
    // Range with guard condition
    public fun test_range_with_guard(x: u64, flag: bool): u64 {
        match (x) {
            1..10 if flag => 1,
            1..10 => 2,
            _ => 0,
        }
    }

    enum E has drop {
        V1(u64),
        V2,
    }

    // Range in enum with guard
    fun enum_range_guard(e: E, flag: bool): u64 {
        match (e) {
            E::V1(0..100) if flag => 1,
            E::V1(0..100) => 2,
            E::V1(_) => 3,
            E::V2 => 4,
        }
    }

    public fun test_enum_guard_true(): u64 {
        enum_range_guard(E::V1(50), true)
    }

    public fun test_enum_guard_false(): u64 {
        enum_range_guard(E::V1(50), false)
    }

    public fun test_enum_out_of_range(): u64 {
        enum_range_guard(E::V1(200), true)
    }

    public fun test_enum_v2(): u64 {
        enum_range_guard(E::V2, true)
    }
}

//# run 0xc0ffee::m::test_range_with_guard --args 5u64 true

//# run 0xc0ffee::m::test_range_with_guard --args 5u64 false

//# run 0xc0ffee::m::test_range_with_guard --args 50u64 true

//# run 0xc0ffee::m::test_enum_guard_true

//# run 0xc0ffee::m::test_enum_guard_false

//# run 0xc0ffee::m::test_enum_out_of_range

//# run 0xc0ffee::m::test_enum_v2
