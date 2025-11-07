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

    public fun test6(x: i8) {
        if (x + 1 > 127) { bar() };
    }

    public fun test7(x: &i8, y: &i8) {
        if ((*x + *y > 127) == true) { bar() };
    }

        // 8-bit
    const I8_MIN: i8 = -128;
    const I8_MAX: i8 = 127;

    // 16-bit
    const I16_MIN: i16 = -32768;
    const I16_MAX: i16 = 32767;

    // 32-bit
    const I32_MIN: i32 = -2147483648;
    const I32_MAX: i32 = 2147483647;

    // 64-bit
    const I64_MIN: i64 = -9223372036854775808;
    const I64_MAX: i64 = 9223372036854775807;

    // 128-bit
    const I128_MIN: i128 = -170141183460469231731687303715884105728;
    const I128_MAX: i128 = 170141183460469231731687303715884105727;

    // 256-bit (custom / nonstandard)
    const I256_MIN: i256 = -57896044618658097711785492504343953926634992332820282019728792003956564819968;
    const I256_MAX: i256 = 57896044618658097711785492504343953926634992332820282019728792003956564819967;

    public fun test8(x: i8) {
        if (x < I8_MIN || I8_MIN > x) { bar() };
        if (foo(x) <= I8_MIN) { bar() };
        if (I8_MIN >= foo(x)) { bar() };
        if (foo(x) > I8_MIN) { bar() };
        if (I8_MIN < foo(x)) { bar() };
        if (foo(x) >= I8_MIN) { bar() };
        if (I8_MIN <= foo(x)) { bar() };
    }

    public fun test9(a: i8, b: i16, c: i32, d: i64, e: i128, f: i256) {
        if (a > I8_MAX) { bar() };
        if (b >= I16_MAX) { bar() };
        if (I32_MAX < c) { bar() };
        if (I64_MAX <= d) { bar() };
        if (e < I128_MAX) { bar() };
        if (f <= I256_MAX) { bar() };
        if (I256_MAX >= f) { bar() };
        if (I128_MAX > e) { bar() };
        spec {
            assert a <= I8_MAX;
        }
    }

    // simulate `abs`
    public fun test_abs(x: i64): i64 {
        // should not warn for `x >= 0`
        if (x >= 0) {
            x
        } else {
            -x
        }
    }

}

module 0xc0ffee::no_warn {
    #[lint::skip(unnecessary_numerical_extreme_comparison)]
    public fun test(x: u8) {
        if (x < 0) abort 1;
    }
}
