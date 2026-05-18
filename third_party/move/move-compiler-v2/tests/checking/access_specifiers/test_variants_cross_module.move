module 0x42::inner {
    // Private enum - cannot be tested from outside
    enum PrivateEnum {
        A,
        B,
    }

    public fun get_enum(): PrivateEnum {
        PrivateEnum::A
    }
}

module 0x42::outer {
    use 0x42::inner;

    public fun test_private_enum(): bool {
        let e = inner::get_enum();
        e is inner::PrivateEnum::A  // Error: testing private enum variant
    }
}
