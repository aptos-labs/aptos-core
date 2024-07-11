module 0x42::test {
    struct S(u8);

    fun foo(x: S) {
        x.0x42;
    }
}
