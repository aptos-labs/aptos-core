//# publish
module 0xc0ffee::m {
    enum Data has drop {
        V1(u8, u8),
        V2(u8, u8, u8)
    }

    fun compute(x: Data, y: u8, z: u8): u8 {
        match ((x, y, z)) {
            (Data::V1(a, b), 1, 2) => a + b + 10,
            (Data::V2(a, b, _c), 5, 6) => a + b,
            _ => 99,
        }
    }

    public fun test(): u8 {
        let d1 = Data::V1(3, 4);
        let d2 = Data::V2(5, 6, 7);
        let r1 = compute(d1, 1, 2);
        let r2 = compute(d2, 5, 7);
        r1 + r2
    }
}

//# run 0xc0ffee::m::test
