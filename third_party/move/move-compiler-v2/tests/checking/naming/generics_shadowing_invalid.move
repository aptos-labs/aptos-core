address 0x2 {

module M {
    struct S has drop {}

    fun foo<S>(s1: S, s2: S): S {
        (s1: Self::S);
        let s: S = S {};
        bar(s1);
        S {}
    }

    fun bar(s: S) {}
}

}
