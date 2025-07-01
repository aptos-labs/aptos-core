//# publish
module 0xCAFE::m0 {
    public struct S has copy, drop {
        x: bool,
    }

    public enum E has copy, drop {
        E1,
    }
}

//# publish
module 0xCAFE::test_m0 {
    use 0xCAFE::m0::S;
    use 0xCAFE::m0::E;

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

//# run 0xCAFE::test_m0::test

//# publish
module 0xCAFE::m1 {
    public struct S has copy, drop {
        x: bool,
    }

    public enum E has copy, drop {
        E1,
    }

}

//# publish
module 0xCAFE::test_m1 {
    use 0xCAFE::m1::S;

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

//# run 0xCAFE::test_m1::test
