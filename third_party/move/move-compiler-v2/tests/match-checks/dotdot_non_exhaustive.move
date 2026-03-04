module 0x42::test {
    struct S1(u8) has copy, drop;

    enum E1 has drop {
        A(u8, bool),
        B(u8),
        C { x: u8, y: S1 },
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
