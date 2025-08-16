//# publish
module 0x42::test {
    public enum Foo has drop {
        A(u8),
        B(u8),
    }
}

//# publish
module 0x42::test_common_access {
    use 0x42::test::Foo;

    fun common_access(x: Foo): u8 {
        x.0
    }

    fun test_common_access(): u8 {
        let x = Foo::A(42);
        common_access(x)
    }
}

//# run --verbose -- 0x42::test_common_access::test_common_access
