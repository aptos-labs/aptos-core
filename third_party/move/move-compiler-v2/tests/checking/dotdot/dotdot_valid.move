module 0x42::test {
    struct S0() has copy;

    struct S1(u8) has copy;

    struct S2(bool, S0) has copy;

    struct S3 has copy {
        x: bool,
        y: u8
    }

    struct S4<T> has copy {
        x: T,
        y: S3
    }

    enum E1 has drop {
        A(u8, bool),
        B(u8),
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
    }

    fun simple_2_ref(x: &S2) {
        let S2(..) = x;
        let S2(_x, ..) = x;
        let S2(.., _x) = x;
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

    fun nested(x: S4<bool>) {
        let S4 { x: _x, .. } = x;
        let S4 { y: _y, .. } = x;
        let S4 { y: S3 { .. }, .. } = x;
        let S4 { y: S3 { x: _x, .. }, .. } = x;
        let S4 { y: S3 { x: _x1, .. }, x: _x2 } = x;
        let S4 { y: S3 { y: _y, .. }, .. } = x;
        let S4 { y: S3 { x: _x1, .. }, x: _x2 } = x;
    }

    fun nested_ref(x: &S4<bool>) {
        let S4 { x: _x, .. } = x;
        let S4 { y: _y, .. } = x;
        let S4 { y: S3 { .. }, .. } = x;
        let S4 { y: S3 { x: _x, .. }, .. } = x;
        let S4 { y: S3 { x: _x1, .. }, x: _x2 } = x;
        let S4 { y: S3 { y: _y, .. }, .. } = x;
        let S4 { y: S3 { x: _x1, .. }, x: _x2 } = x;
    }

    fun simple_4(x: E1): u8 {
        match (x) {
            E1::A(x, ..) => {
                x
            }
            E1::B(x) => {
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
}
