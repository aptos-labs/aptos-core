//# publish
module 0xCAFE::Module0 {
    enum Enum0 has copy, drop {
        E1(bool),
    }

    struct S has copy, drop {
        x: Enum0,
    }

    fun f(s: S) {
        let a: || has copy+drop = || {};
        *(match (s.x) {
            Enum0::E1(_) => &mut a,
        }) = || {};
    }
}
