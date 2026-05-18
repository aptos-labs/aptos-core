//# publish
module 0xc0ffee::m {
    enum Data has drop, copy {
        V1 { x: u64 },
        V2 { x: u64 },
    }

    // Mixed tuple: (&Data, u64). The enum part is matched by pattern,
    // and the guard references both the enum's inner field (by ref) and the primitive.
    fun match_ref(d: &Data, p: u64): u64 {
        match ((d, p)) {
            (Data::V1 { x }, y) if (*x > 10 && y > 5) => *x + y,
            (Data::V1 { x }, _) => *x,
            (Data::V2 { x }, y) if (*x == y) => *x * 2,
            _ => 0,
        }
    }

    // V1{20}, p=10: guard *x>10 && y>5 passes => 20+10 = 30
    public fun test_v1_guard_pass(): u64 {
        let d = Data::V1 { x: 20 };
        match_ref(&d, 10)
    }

    // V1{20}, p=3: guard fails (3 <= 5), falls to V1 without guard => 20
    public fun test_v1_guard_fail(): u64 {
        let d = Data::V1 { x: 20 };
        match_ref(&d, 3)
    }

    // V2{7}, p=7: guard *x == y passes => 7*2 = 14
    public fun test_v2_guard_pass(): u64 {
        let d = Data::V2 { x: 7 };
        match_ref(&d, 7)
    }

    // V2{7}, p=8: guard fails, falls to wildcard => 0
    public fun test_v2_guard_fail(): u64 {
        let d = Data::V2 { x: 7 };
        match_ref(&d, 8)
    }
}

//# run 0xc0ffee::m::test_v1_guard_pass

//# run 0xc0ffee::m::test_v1_guard_fail

//# run 0xc0ffee::m::test_v2_guard_pass

//# run 0xc0ffee::m::test_v2_guard_fail
