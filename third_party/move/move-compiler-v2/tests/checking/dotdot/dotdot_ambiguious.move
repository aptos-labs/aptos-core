module 0x42::test {
    struct S(bool, u8, address);

    fun simple_0(x: S) {
        let S(.., x, ..) = x;
    }
}
