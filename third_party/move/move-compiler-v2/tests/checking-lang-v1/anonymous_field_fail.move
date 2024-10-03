module 0x42::test {
    struct S(u8);

    enum E {
        V(S)
    }

    fun foo(x: S, y: E) {
        x.1;
        let S(_x) = x;
        match (y) {
            E::V(_y) => {}
        }
    }
}
