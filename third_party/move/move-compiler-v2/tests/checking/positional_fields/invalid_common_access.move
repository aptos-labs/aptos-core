module 0x42::test {
    enum Foo has drop {
        A(u8, bool),
        B(u8, u8),
    }

    fun common_access(x: Foo): u8 {
        x.1
    }
}
