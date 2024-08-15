module 0x42::test {
    struct S(bool, u8, address);

    fun extra_dotdot(x: S) {
        let S(x, _, _, ..) = x;
    }
}
