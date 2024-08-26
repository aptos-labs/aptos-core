module 0x42::test {
    struct S(u8, bool);

    fun foo() {
        S { _0: 42, _1: 42 };
    }
}
