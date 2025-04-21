//# publish
module 0x42::test {
    struct S0 has copy, drop {
        x: u8,
    }

    struct S1(u64, bool) has drop;

    struct S2(S0, u8) has copy, drop;

    struct S3(S2, S0, S2) has drop;

    enum E has drop {
        V1(u8, bool),
        V2(S3),
    }

    fun simple(x: S1): S1 {
        x.0 = 42;
        x.1 = true;
        x
    }

    fun simple_ref(x: &mut S1) {
        x.0 = 42;
        x.1 = true;
    }

    fun assign0(a: u64, b: bool): S1 {
        let x = S1(a, b);
        while (x.1) {
            x = S1(x.0 - 1, x.0 >= 2);
        };
        x
    }

    fun assign1(x: S1): u64 {
        let count = 0;
        while (x.1) {
            let y = if (x.0 > 0) { x.0 - 1 } else { 0 };
            x = S1(y, y >=1);
            count = count + 1;
        };
        count
    }

    fun assign_chained(x: S3): S3 {
        x.0.0.x + x.1.x + x.2.0.x;
        x.0.0.x = 0;
        x.1.x = 1;
        x.2.0.x = 2;
        x
    }

    fun assign_enum(x: &mut E) {
        match (x) {
            E::V1(x, y) => {
                *x = 42;
                *y = true;
            },
            E::V2(x) => {
                x.0.0.x = 0;
                x.1.x = 1;
                x.2.0.x = 2;
            }
        }
    }

    fun test_simple(): S1 {
        let x = S1(0, false);
        let y = simple(x);
        y
    }

    fun test_simple_ref(): S1 {
        let x = S1(0, false);
        simple_ref(&mut x);
        x
    }

    fun test_assign0(): S1 {
        assign0(4, true)
    }

    fun test_assign1(): u64 {
        let x = S1(4, true);
        assign1(x)
    }

    fun test_assign_chained(): S3 {
        let x0 = S0 { x: 42 };
        let x1 = S2(x0, 42);
        let x2 = S3(x1, x0, x1);
        x2 = assign_chained(x2);
        x2
    }

    fun test_assign_enum_1(): bool {
        let x0 = S0 { x: 43 };
        let x1 = S2(x0, 42);
        let x2 = S3(x1, x0, x1);
        let x3 = E::V2(x2);
        assign_enum(&mut x3);
        let y0 = S0 { x: 0 };
        let y1 = S0 { x: 1 };
        let y2 = S0 { x: 2 };
        let y3 = S2(y0, 42);
        let y4 = S2(y2, 42);
        let y5 = S3(y3, y1, y4);
        let y6 = E::V2(y5);
        x3 == y6
    }

    fun test_assign_enum_2(): bool {
        let x = E::V1(0, false);
        assign_enum(&mut x);
        let y = E::V1(42, true);
        x == y
    }
}

//# run --verbose -- 0x42::test::test_simple

//# run --verbose -- 0x42::test::test_simple_ref

//# run --verbose -- 0x42::test::test_assign0

//# run --verbose -- 0x42::test::test_assign1

//# run --verbose -- 0x42::test::test_assign_chained

//# run --verbose -- 0x42::test::test_assign_enum_1

//# run --verbose -- 0x42::test::test_assign_enum_2
