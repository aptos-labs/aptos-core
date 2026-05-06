// @checks=experimental
module 0xc0ffee::m {
    package fun foo(): u64 {
        42
    }

    public fun bar(): u64 {
        foo()
    }
}

module 0xc0ffee::n {
    #[test_only]
    use 0xc0ffee::m;

    #[test]
    fun test_foo() {
        assert!(m::foo() == 42, 1);
    }
}
