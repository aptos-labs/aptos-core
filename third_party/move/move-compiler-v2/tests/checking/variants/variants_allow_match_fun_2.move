module 0x815::m {
    enum CommonFields {
        Foo{x: u64, y: u8},
        Bar{x: u64, z: u32}
    }

    fun match(): bool {
        let c = CommonFields::Foo{x: 0, y: 0};
        match (c) {
            Foo{x, y: _} => x > 0,
            _ => false
        }
    }

    fun caller(): bool {
        match()
    }
}
