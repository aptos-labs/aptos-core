//# publish
module 0xCAFE::Module0 {
    struct S has copy, drop ;
    enum E has copy, drop {
        V1 { f1: u8, f2: S, },
        V2,
    }
    public fun f() {
         match (E::V2) {
            E::V1 { f2: S, .. } => &(*(&true) && true),
            _ => &(*(&true) && true),
        };
    }
}
