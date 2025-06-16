//# publish
module 0xCAFE::m0 {
    struct S has copy, drop {
        x: bool,
    }

    enum E has copy, drop {
        E1,
    }

    fun f(s: S) {
        *(match (E::E1) {
            E::E1 => {
                *(&mut (true)) = s.x;
                &mut ( 0u8)
            }
        }) = 123u8;
    }

    fun test() {
        let s = S { x: true };
        f(s);
    }
}

//# run 0xCAFE::m0::test

//# publish
module 0xCAFE::m1 {
    struct S has copy, drop {
        x: bool,
    }

    enum E has copy, drop {
        E1,
    }

    fun f(s: S, y: u64) {
        *({
            if (y == 10) {
                abort 0;
            };
            *(&mut s.x) = s.x;
            let z = 3;
            &mut z
        }) = 123u8;
    }

    fun test() {
        let s = S { x: true };
        f(s, 20);
    }
}

//# run 0xCAFE::m1::test
