//# publish
module 0xCAFE::Module0 {
    public enum Enum0 has copy, drop {
        E1(bool),
    }

    public struct S has copy, drop {
        x: Enum0,
    }

}

//# publish
module 0xCAFE::test_Module0 {
    use 0xCAFE::Module0::S;
    use 0xCAFE::Module0::Enum0;

    fun f(s: S) {
        let a: || has copy+drop = || {};
        *(match (s.x) {
            Enum0::E1(_) => &mut a,
        }) = || {};
    }
}
