module 0x815::m {
    enum CommonFields {
        Foo{x: u64, y: u8},
        Bar{x: u64, z: u32}
    }

    fun match(c: CommonFields, t: u64): bool {
        match (c) {
            Foo{x, y: _} => x > t,
            _ => false
        }
    }

    fun caller(c: CommonFields): bool {
        match(c, 22) && true
    }
}
