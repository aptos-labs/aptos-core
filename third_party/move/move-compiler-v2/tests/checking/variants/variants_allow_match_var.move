module 0x815::m {
    enum CommonFields {
        Foo{x: u64, y: u8},
        Bar{x: u64, z: u32}
    }

    fun match(c: CommonFields, t: u64): bool {
        match (c) {
            Foo{x, y: _} => {
                let match = x > t;
                match
            }
            _ => false
        }
    }
}
