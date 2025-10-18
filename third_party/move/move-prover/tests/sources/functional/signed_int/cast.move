module 0x42::TestCast {

    spec module {
        pragma verify = true;
    }

    // --------------
    // Type promotion
    // --------------

    fun i8_cast(x: i8): i64 {
        (x as i64)
    }
    spec i8_cast {
        aborts_if false;
    }

    fun i16_cast(x: i16): i64 {
        (x as i64)
    }
    spec i16_cast {
        aborts_if false;
    }

    fun i32_cast(x: i32): i64 {
        (x as i64)
    }
    spec i32_cast {
        aborts_if false;
    }

    fun i64_cast(x: i64): i128 {
        (x as i128)
    }
    spec i64_cast {
        aborts_if false;
    }

    fun i128_cast(x: i128): i256 {
        (x as i256)
    }
    spec i128_cast {
        aborts_if false;
    }

    // -------------
    // Type demotion
    // -------------

    fun aborting_i8_cast_incorrect(x: i64): i8 {
        (x as i8)
    }
    spec aborting_i8_cast_incorrect {
        aborts_if false;
    }

    fun aborting_i8_cast(x: i64): i8 {
        (x as i8)
    }
    spec aborting_i8_cast {
        aborts_if x > 127 || x < -128;
    }

    fun aborting_i16_cast_incorrect(x: i64): i16 {
        (x as i16)
    }
    spec aborting_i16_cast_incorrect {
        aborts_if false;
    }

    fun aborting_i16_cast(x: i64): i16 {
        (x as i16)
    }
    spec aborting_i16_cast {
        aborts_if x > 32767 || x < -32768;
    }

    fun aborting_i32_cast_incorrect(x: i64): i32 {
        (x as i32)
    }
    spec aborting_i32_cast_incorrect {
        aborts_if false;
    }

    fun aborting_i32_cast(x: i64): i32 {
        (x as i32)
    }
    spec aborting_i32_cast {
        aborts_if x > 2147483647 || x < -2147483648;
    }

    fun aborting_i64_cast_incorrect(x: i128): i64 {
        (x as i64)
    }
    spec aborting_i64_cast_incorrect {
        aborts_if false;
    }

    fun aborting_i64_cast(x: i128): i64 {
        (x as i64)
    }
    spec aborting_i64_cast {
        aborts_if x > 9223372036854775807 || x < -9223372036854775808;
    }

    fun aborting_i128_cast_incorrect(x: i256): i128 {
        (x as i128)
    }
    spec aborting_i128_cast_incorrect {
        aborts_if false;
    }

    fun aborting_i128_cast(x: i256): i128 {
        (x as i128)
    }
    spec aborting_i128_cast {
        aborts_if x > 170141183460469231731687303715884105727 || x < -170141183460469231731687303715884105728;
    }

    fun cast_bv_to_sint(x: u64): i64 {
       (x & 0x1u64) as i64
    }

    spec cast_bv_to_sint {
        aborts_if false;
    }
}
