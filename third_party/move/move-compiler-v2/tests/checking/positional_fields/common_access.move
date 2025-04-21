module 0x42::test {
    enum Foo has drop {
        A(u8),
        B(u8),
    }

    fun common_access(x: Foo): u8 {
        x.0
    }
}
