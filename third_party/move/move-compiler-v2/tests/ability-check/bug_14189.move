module 0x42::test {
    struct S0 has copy, drop {
        x: u8,
    }

    struct S1 has drop {
        x: u64,
        y: bool,
    }

    struct S2 has drop {
        x: S0,
        y: u8,
    }

    struct S3 has drop {
        x: S2,
        y: S0,
        z: S2,
    }

    fun assign_chained(x: S3): S3 {
        x.x.x.x + x.y.x + x.z.x.x;
        x.x.x.x = 0;
        x.y.x = 1;
        x.z.x.x = 2;
        x
    }

    fun test_assign_chained(): S3 {
        let x0 = S0 { x: 42 };
        let x1 = S2 { x: x0, y: 42 };
        // x1: S2 where S2 is not copy
        let x2 = S3 { x: x1, y: x0, z: x1 };
        x2 = assign_chained(x2);
        x2
    }
}
