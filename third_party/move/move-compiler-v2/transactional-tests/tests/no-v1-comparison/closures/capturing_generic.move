//# publish
module 0x42::test {
    struct S has drop, copy {
        f: u64
    }

    fun eq_with<T: drop>(x: T): |T| bool {
        |y| x == y
    }

    public fun test_u64(x: u64): bool {
        eq_with(2)(x)
    }

    public fun test_S(x: u64): bool {
        eq_with(S{f:2})(S{f: x})
    }

}

//# run 0x42::test::test_u64 --args 3

//# run 0x42::test::test_S --args 2
