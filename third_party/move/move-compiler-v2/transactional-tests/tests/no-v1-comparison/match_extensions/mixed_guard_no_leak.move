//# publish
module 0xc0ffee::m {
    enum Data has drop, copy {
        V1 { z: u64 },
        V2 { z: u64 },
    }

    // Mixed tuple: (&Data, u64).  The primitive-position pattern `y` binds
    // y = $prim_0 = x (NOT param y).  Guard is always false, so we fall
    // through to the catch-all whose body `y` must see parameter y, not x.
    fun mixed_no_leak(d: &Data, x: u64, y: u64): u64 {
        match ((d, x)) {
            (Data::V1 { z }, y) if false => *z + y,
            _ => y,
        }
    }

    // Same idea but the non-leaking arm and the catch-all both exist.
    fun mixed_no_leak_multi(d: &Data, x: u64, y: u64): u64 {
        match ((d, x)) {
            (Data::V1 { z }, y) if false => *z + y,
            (Data::V2 { z }, _) => y + *z,
            _ => y,
        }
    }

    // V1{5}, x=10, y=20: first arm guard fails, catch-all returns y=20
    public fun test_v1_fallthrough(): u64 {
        let d = Data::V1 { z: 5 };
        mixed_no_leak(&d, 10, 20)
    }

    // V2{3}, x=10, y=20: first arm wrong variant, catch-all returns y=20
    public fun test_v2_fallthrough(): u64 {
        let d = Data::V2 { z: 3 };
        mixed_no_leak(&d, 10, 20)
    }

    // V1{5}, x=10, y=20: multi - first arm fails, second wrong variant, catch-all => y=20
    public fun test_multi_v1(): u64 {
        let d = Data::V1 { z: 5 };
        mixed_no_leak_multi(&d, 10, 20)
    }

    // V2{3}, x=10, y=20: multi - first arm wrong variant, second matches => y+z = 20+3 = 23
    public fun test_multi_v2(): u64 {
        let d = Data::V2 { z: 3 };
        mixed_no_leak_multi(&d, 10, 20)
    }
}

//# run 0xc0ffee::m::test_v1_fallthrough

//# run 0xc0ffee::m::test_v2_fallthrough

//# run 0xc0ffee::m::test_multi_v1

//# run 0xc0ffee::m::test_multi_v2
