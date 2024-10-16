module 0x42::test {
    struct S0() has copy;

    struct S1(u8) has copy, drop;

    struct S2(bool, S0) has copy;

    struct S3 has copy {
        x: bool,
        y: u8
    }

    struct S4<T> has copy {
        x: T,
        y: S3
    }

    struct S5<T, U>(T, U);

    struct S6<T, U> {
        x: T,
        y: U
    }

    struct S7(u8, u16, u32, u64);

    enum E1 has drop {
        A(u8, bool),
        B(u8),
        C { x: u8, y: S1 },
    }

    fun simple_0(x: S0) {
        let S0(..) = x;
    }

    fun simple_0_ref(x: &S0) {
        let S0(..) = x;
    }

    fun simple_1(x: S1) {
        let S1(..) = x;
    }

    fun simple_1_ref(x: &mut S1) {
        let S1(..) = x;
    }

    fun simple_2(x: S2) {
        let S2(..) = x;
        let S2(_x, ..) = x;
        let S2(.., _x) = x;
        let S2(.., _) = x;
        let S2(_, ..) = x;
        let S2(_x, _y, ..) = x;
        let S2(_x, .., _y) = x;
        let S2(.., _x, _y) = x;
    }

    fun simple_2_ref(x: &S2) {
        let S2(..) = x;
        let S2(_x, ..) = x;
        let S2(.., _x) = x;
        let S2(.., _) = x;
        let S2(_, ..) = x;
        let S2(_x, _y, ..) = x;
        let S2(_x, .., _y) = x;
        let S2(.., _x, _y) = x;
    }

    fun simple_3(x: S3) {
        let S3 { .. } = x;
        let S3 { x: _x, .. } = x;
        let S3 { y: _y, .. } = x;
    }

    fun simple_3_ref(x: S3) {
        let S3 { .. } = x;
        let S3 { x: _x, .. } = x;
        let S3 { y: _y, .. } = x;
    }

    fun nested1(x: S4<bool>) {
        let S4 { x: _x, .. } = x;
        let S4 { y: _y, .. } = x;
        let S4 { y: S3 { .. }, .. } = x;
        let S4 { y: S3 { x: _x, .. }, .. } = x;
        let S4 { y: S3 { x: _x1, .. }, x: _x2 } = x;
        let S4 { y: S3 { y: _y, .. }, .. } = x;
        let S4 { y: S3 { x: _x1, .. }, x: _x2 } = x;
    }

    fun nested1_ref(x: &S4<bool>) {
        let S4 { x: _x, .. } = x;
        let S4 { y: _y, .. } = x;
        let S4 { y: S3 { .. }, .. } = x;
        let S4 { y: S3 { x: _x, .. }, .. } = x;
        let S4 { y: S3 { x: _x1, .. }, x: _x2 } = x;
        let S4 { y: S3 { y: _y, .. }, .. } = x;
        let S4 { y: S3 { x: _x1, .. }, x: _x2 } = x;
    }

    fun nested2(x: S5<bool, S1>) {
        let S5(.., S1(..)) = x;
    }

    fun nested2_ref(x: &S5<bool, S1>) {
        let S5(.., S1(..)) = x;
    }

    fun nested3(x: S5<bool, S4<bool>>) {
        let S5(.., S4 { .. }) = x;
    }

    fun nested3_ref(x: &S5<bool, S4<bool>>) {
        let S5(.., S4 { .. }) = x;
    }

    fun nested4(x: S4<S1>) {
        let S4 { x: S1(..), .. } = x;
    }

    fun nested4_ref(x: &S4<S1>) {
        let S4 { x: S1(..), .. } = x;
    }

    fun simple_4(x: E1): u8 {
        match (x) {
            E1::A(x, ..) => {
                x
            },
            E1::B(x) => {
                x
            },
            E1::C { x, .. } => {
                x
            }
        }
    }

    fun simple_4_ref(x: &E1): &u8 {
        match (x) {
            E1::A(x, ..) => {
                x
            }
            E1::B(x) => {
                x
            }
        }
    }

    fun simple_5(x: E1): u8 {
        match (x) {
            E1::A(.., y) => {
                if (y) {
                    1
                } else {
                    0
                }
            },
            E1::B(x) => {
                x
            },
            E1::C { y: S1(x), .. } => {
                x
            }
        }
    }

    fun simple_6(x: &S7) {
        let S7(_w, .., _z) = x;
        let S7(_w, _x, .., _y, _z) = x;
    }

    inline fun lambda_param(f: |S2| bool): bool {
        let x = S2(true, S0());
        f(x)
    }

    fun test_lambda_param(): bool {
        lambda_param(|S2(x, ..)| x)
    }
}
