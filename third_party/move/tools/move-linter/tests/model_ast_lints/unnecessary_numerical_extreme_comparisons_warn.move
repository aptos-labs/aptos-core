module 0xc0ffee::m {
    fun bar() {}

    fun foo<T>(x: T): T {
        x
    }

    public fun test1(x: u8) {
        if (x + 1 > 255) { bar() };
    }

    public fun test2(x: &u8, y: &u8) {
        if ((*x + *y > 255) == true) { bar() };
    }

    public fun test3(x: u8) {
        if (x < 0 || 0 > x) { bar() };
        if (foo(x) <= 0) { bar() };
        if (0 >= foo(x)) { bar() };
        if (foo(x) > 0) { bar() };
        if (0 < foo(x)) { bar() };
        if (foo(x) >= 0) { bar() };
        if (0 <= foo(x)) { bar() };
    }

    const U8_MAX: u8 = 255;
    const U16_MAX: u16 = 65535;
    const U32_MAX: u32 = 4294967295;
    const U64_MAX: u64 = 18446744073709551615;
    const U128_MAX: u128 = 340282366920938463463374607431768211455;
    const U256_MAX: u256 =
        115792089237316195423570985008687907853269984665640564039457584007913129639935;

    public fun test4(a: u8, b: u16, c: u32, d: u64, e: u128, f: u256) {
        // Should not warn for `f > (U8_MAX as u256)`.
        if (a > U8_MAX || f > (U8_MAX as u256)) { bar() };
        if (b >= U16_MAX) { bar() };
        if (U32_MAX < c) { bar() };
        if (U64_MAX <= d) { bar() };
        if (e < U128_MAX) { bar() };
        if (f <= U256_MAX) { bar() };
        if (U256_MAX >= f) { bar() };
        if (U128_MAX > e) { bar() };
        spec {
            assert a <= U8_MAX;
        }
    }

    inline fun apply(f: |u8|bool, x: u8): bool {
        f(x)
    }

    public fun test5(x: u8): bool {
        apply(|x| x > U8_MAX, x)
    }
}

module 0xc0ffee::no_warn {
    #[lint::skip(unnecessary_numerical_extreme_comparison)]
    public fun test(x: u8) {
        if (x < 0) abort 1;
    }
}
