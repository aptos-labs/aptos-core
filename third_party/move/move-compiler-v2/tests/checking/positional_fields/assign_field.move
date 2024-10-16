module 0x42::test {
    struct S0 {
        x: u8,
    }

    struct S1(u64, bool);

    struct S2(S0, u8);

    struct S3(S2, S0, S2);

    enum E {
        V1(u8, bool),
        V2(S3),
    }

    fun simple(x: S1) {
        x.0 = 42;
        x.1 = true;
    }

    fun simple_ref(x: &mut S1) {
        x.0 = 42;
        x.1 = true;
    }

    fun assign0 (a: u64, b: bool) {
        let x = S1(a, b);
        while (x.1) {
                x = S1(x.0 - 1, x.0 >= 1);
            }
    }

    fun assign1 (x: S1): u64 {
        let count = 0;
        while (x.1) {
            let y = if (x.0 > 0) { x.0 - 1 } else { 0 };
            x = S1(y, y >=1);
            count = count + 1;
        };
        count
    }

    fun assign_chained(x: S3) {
        x.0.0.x + x.1.x + x.2.0.x;
        x.0.0.x = 0;
        x.1.x = 1;
        x.2.0.x = 2;
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
}
