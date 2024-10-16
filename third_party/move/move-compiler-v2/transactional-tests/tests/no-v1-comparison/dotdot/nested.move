//# publish
module 0x42::test {
    struct S0<A, B>(A, B) has drop;

    struct S1<A, B> has drop {
        x: A,
        y: B
    }

    enum E has drop {
        V1(u8, S1<u8, bool>),
        V2 {
            x: u8,
            y: S0<u8, bool>
        }
    }

    fun extract_first_u8(x: &E): u8 {
        match (x) {
            E::V1(a, ..) => *a,
            E::V2 { x, .. } => *x,
        }
    }

    fun extract_last_u8(x: &E): u8 {
        match (x) {
            E::V1(.., S1 { x, ..}) => *x,
            E::V2 { y: S0(x, ..), .. } => *x,
        }
    }

    fun test1(): u8 {
        let x = E::V1(42, S1 { x: 43, y: true });
        extract_first_u8(&x)
    }

    fun test2(): u8 {
        let x = E::V2 { x: 42, y: S0(43, true) };
        extract_first_u8(&x)
    }

    fun test3(): u8 {
        let x = E::V1(42, S1 { x: 43, y: true });
        extract_last_u8(&x)
    }

    fun test4(): u8 {
        let x = E::V2 { x: 42, y: S0(43, true) };
        extract_last_u8(&x)
    }
}

//# run --verbose -- 0x42::test::test1

//# run --verbose -- 0x42::test::test2

//# run --verbose -- 0x42::test::test3

//# run --verbose -- 0x42::test::test4
