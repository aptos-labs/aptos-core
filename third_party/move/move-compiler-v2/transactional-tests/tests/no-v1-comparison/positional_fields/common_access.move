//# publish
module 0x42::test {
    enum Foo has drop {
        A(u8),
        B(u8),
    }

    fun common_access(x: Foo): u8 {
        x.0
    }

    fun test_common_access(): u8 {
        let x = Foo::A(42);
        common_access(x)
    }
}

//# run --verbose -- 0x42::test::test_common_access
