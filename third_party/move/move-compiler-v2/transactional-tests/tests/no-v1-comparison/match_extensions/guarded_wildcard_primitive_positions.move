//# publish
module 0xc0ffee::guarded_wildcard_primitive_positions {
    enum E has drop {
        V1 { f: u8 },
        V2,
    }

    fun match_guarded(enum_val: E, flag: bool): u8 {
        match ((enum_val, flag)) {
            (E::V1 { .. }, _) if flag => 1,
            _ => 0,
        }
    }

    public fun test_guard_true(): u8 {
        match_guarded(E::V1 { f: 7 }, true)
    }

    public fun test_guard_false(): u8 {
        match_guarded(E::V1 { f: 7 }, false)
    }

    public fun test_guard_other_variant(): u8 {
        match_guarded(E::V2, true)
    }
}

//# run 0xc0ffee::guarded_wildcard_primitive_positions::test_guard_true

//# run 0xc0ffee::guarded_wildcard_primitive_positions::test_guard_false

//# run 0xc0ffee::guarded_wildcard_primitive_positions::test_guard_other_variant
