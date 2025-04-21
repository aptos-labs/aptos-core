module 0x42::test {
    struct S(u8, bool);

    fun foo() {
        S { 0: 42, 1: 42 };
    }
}
