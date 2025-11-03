//# publish
module 0x66::m {
    fun compile_for_testing() {
        // We are NOT compiling for testing, so expect this to be false. See independent
        // compile_for_testing test.
        assert!(!__COMPILE_FOR_TESTING__, 66);
    }

    fun min_max() {
        assert!(MAX_U8 == 255, 1);
        assert!(MAX_U16 == 65535, 1);
        assert!(MAX_U32 == 4294967295, 1);
        assert!(MAX_U64 == 18446744073709551615, 1);
        assert!(MAX_U128 == 340282366920938463463374607431768211455, 1);
        assert!(MAX_U256 == 115792089237316195423570985008687907853269984665640564039457584007913129639935, 1);

        assert!(MIN_I8 == -128, 1);
        assert!(MAX_I8 == 127, 1);
        assert!(MIN_I16 == -32768, 1);
        assert!(MAX_I16 == 32767, 1);
        assert!(MIN_I32 == -2147483648, 1);
        assert!(MAX_I32 == 2147483647, 1);
        assert!(MIN_I64 == -9223372036854775808, 1);
        assert!(MAX_I64 == 9223372036854775807, 1);
        assert!(MIN_I128 == -170141183460469231731687303715884105728, 1);
        assert!(MAX_I128 == 170141183460469231731687303715884105727, 1);
        assert!(MIN_I256 == -57896044618658097711785492504343953926634992332820282019728792003956564819968, 1);
        assert!(MAX_I256 == 57896044618658097711785492504343953926634992332820282019728792003956564819967, 1);
    }
}

//# run 0x66::m::compile_for_testing

//# run 0x66::m::min_max

//# publish
module 0x66::shadow {
    const MAX_U8: bool = false;

    fun shadowed() {
        assert!(!MAX_U8, 66);
    }
}

//# run 0x66::shadow::shadowed
