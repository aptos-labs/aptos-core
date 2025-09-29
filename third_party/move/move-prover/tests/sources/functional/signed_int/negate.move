module 0x42::TestCast {

    spec module {
        pragma verify = true;
    }

    // i8
    fun negate_i8_incorrect(x: i8): i8 {
        -x
    }
    spec negate_i8_incorrect {
        aborts_if false;
    }

    fun negate_i8(x: i8): i8 {
        -x
    }
    spec negate_i8 {
        aborts_if x == -128; // -(2^7)
    }

    // i16
    fun negate_i16_incorrect(x: i16): i16 {
        -x
    }
    spec negate_i16_incorrect {
        aborts_if false;
    }

    fun negate_i16(x: i16): i16 {
        -x
    }
    spec negate_i16 {
        aborts_if x == -32768; // -(2^15)
    }


    // i32
    fun negate_i32_incorrect(x: i32): i32 {
        -x
    }
    spec negate_i32_incorrect {
        aborts_if false;
    }

    fun negate_i32(x: i32): i32 {
        -x
    }
    spec negate_i32 {
        aborts_if x == -2147483648; // -(2^31)
    }


    // i64
    fun negate_i64_incorrect(x: i64): i64 {
        -x
    }
    spec negate_i64_incorrect {
        aborts_if false;
    }

    fun negate_i64(x: i64): i64 {
        -x
    }
    spec negate_i64 {
        aborts_if x == -9223372036854775808; // -(2^63)
    }


    // i128
    fun negate_i128_incorrect(x: i128): i128 {
        -x
    }
    spec negate_i128_incorrect {
        aborts_if false;
    }

    fun negate_i128(x: i128): i128 {
        -x
    }
    spec negate_i128 {
        aborts_if x == -170141183460469231731687303715884105728; // -(2^127)
    }


    // i256
    fun negate_i256_incorrect(x: i256): i256 {
        -x
    }
    spec negate_i256_incorrect {
        aborts_if false;
    }

    fun negate_i256(x: i256): i256 {
        -x
    }
    spec negate_i256 {
        aborts_if x == -57896044618658097711785492504343953926634992332820282019728792003956564819968; // -(2^255)
    }
}
