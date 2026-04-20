//# publish
module 0xc0ffee::m {
    enum Color has drop {
        RGB(u8, u8, u8),
        Named(u8),
    }

    fun classify_color(c: Color): u64 {
        match (c) {
            Color::RGB(0..128, 0..128, 0..128) => 1,
            Color::RGB(_, _, _) => 2,
            Color::Named(_) => 3,
        }
    }

    public fun test_dark_color(): u64 {
        classify_color(Color::RGB(50, 50, 50))
    }

    public fun test_bright_color(): u64 {
        classify_color(Color::RGB(200, 200, 200))
    }

    public fun test_mixed_color(): u64 {
        classify_color(Color::RGB(50, 200, 50))
    }

    public fun test_named_color(): u64 {
        classify_color(Color::Named(1))
    }

    enum E has drop {
        V1(u64),
        V2(u64, u64),
        V3,
    }

    fun mixed_patterns(e: E): u64 {
        match (e) {
            E::V1(0..100) => 1,
            E::V1(_) => 2,
            E::V2(0..50, 42) => 3,
            E::V2(_, _) => 4,
            E::V3 => 5,
        }
    }

    public fun test_v1_in_range(): u64 {
        mixed_patterns(E::V1(50))
    }

    public fun test_v1_out_range(): u64 {
        mixed_patterns(E::V1(200))
    }

    public fun test_v2_both_match(): u64 {
        mixed_patterns(E::V2(25, 42))
    }

    public fun test_v2_range_only(): u64 {
        mixed_patterns(E::V2(25, 99))
    }

    public fun test_v3(): u64 {
        mixed_patterns(E::V3)
    }

    fun multi_variant_range(e: E): u64 {
        match (e) {
            E::V1(1..=10) => 1,
            E::V1(_) => 2,
            E::V2(0..100, 0..100) => 3,
            E::V2(_, _) => 4,
            E::V3 => 5,
        }
    }

    public fun test_multi_v1_in(): u64 {
        multi_variant_range(E::V1(5))
    }

    public fun test_multi_v1_out(): u64 {
        multi_variant_range(E::V1(50))
    }

    public fun test_multi_v2_in(): u64 {
        multi_variant_range(E::V2(50, 50))
    }

    public fun test_multi_v2_out(): u64 {
        multi_variant_range(E::V2(200, 200))
    }

    public fun test_multi_v3(): u64 {
        multi_variant_range(E::V3)
    }
}

//# run 0xc0ffee::m::test_dark_color

//# run 0xc0ffee::m::test_bright_color

//# run 0xc0ffee::m::test_mixed_color

//# run 0xc0ffee::m::test_named_color

//# run 0xc0ffee::m::test_v1_in_range

//# run 0xc0ffee::m::test_v1_out_range

//# run 0xc0ffee::m::test_v2_both_match

//# run 0xc0ffee::m::test_v2_range_only

//# run 0xc0ffee::m::test_v3

//# run 0xc0ffee::m::test_multi_v1_in

//# run 0xc0ffee::m::test_multi_v1_out

//# run 0xc0ffee::m::test_multi_v2_in

//# run 0xc0ffee::m::test_multi_v2_out

//# run 0xc0ffee::m::test_multi_v3
