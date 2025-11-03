module 0x42::TestSintArithmetic {

    spec module {
        pragma verify = true;
    }

    // --------------------------
    // Basic arithmetic operation
    // --------------------------

    // Most of the tests here just ensure that what the bytecode operation does
    // is the same as the spec expressions.

    // succeeds.
	fun add_two_number(x: i64, y: i64): (i64, i64) {
		let res: i64 = x + y;
		let z: i64 = 3;
		(z, res)
	}
	spec add_two_number {
	    aborts_if x + y > MAX_I64 || x + y < MIN_I64;
	    ensures result_1 == 3;
	    ensures result_2 == x + y;
	}

    fun div(x: i64, y: i64): (i64, i64) {
        (x / y, x % y)
    }
    spec div {
        aborts_if y == 0;
        ensures result_1 == x / y;
        ensures result_2 == x % y;
    }

    // succeeds.
	fun multiple_ops(x: i64, y: i64, z: i64): i64 {
		x + y * z
	}
	spec multiple_ops {
        ensures result == x + y * z;
    }

    // succeeds.
	fun bool_ops(a: i64, b: i64): (bool, bool) {
        let c: bool;
        let d: bool;
        c = a > b && a >= b;
        d = a < b || a <= b;
        if (!(c != d)) abort 42;
        (c, d)
    }
    spec bool_ops {
        ensures result_1 <==> (a > b && a >= b);
        ensures result_2 <==> (a < b || a <= b);
    }

    // succeeds.
	fun arithmetic_ops(a: i64): (i64, i64) {
        let c: i64;
        c = (6 + 4 - 1) * 2 / 3 % 4;
        if (c != 2) abort 42;
        (c, a)
    }
    spec arithmetic_ops {
        ensures result_1 == (6 + 4 - 1) * 2 / 3 % 4;
        ensures result_2 == a;
    }

    fun f(x: i64) : i64 {
        x+1
    }
    spec f {
        aborts_if x+1 > MAX_I64;
        ensures result == x+1;
    }

    fun g(x: i64) : i64 {
        x+2
    }
    spec g {
        aborts_if x+2 > MAX_I64;
        ensures result == x+2;
    }

    fun h(b: bool): i64 {
        let x: i64 = 3;
        let y: i64;
        if (b) y=f(x) else y=g(x);
        if (b && y != 4) abort 4;
        if (!b && y != 5) abort 5;
        y
    }
    spec h {
        aborts_if false;
    }


    // ---------
    // Underflow
    // ---------

    // succeeds.
    fun underflow(): i64 {
        let x = -9223372036854775808;
        x - 1
    }
	spec underflow {
	    aborts_if true;
	}


    // ----------------
    // Division by zero
    // ----------------

    // succeeds.
    fun div_by_zero(): i64 {
        let x = 0;
        1 / x
    }
	spec div_by_zero {
	    aborts_if true;
	}

    fun div_by_zero_i64_incorrect(x: i64, y: i64): i64 {
        x / y
    }
    spec div_by_zero_i64_incorrect {
        aborts_if false;
    }

    fun div_by_zero_i64(x: i64, y: i64): i64 {
        x / y
    }
    spec div_by_zero_i64 {
        aborts_if y == 0;
    }


    // --------------------
    // Overflow by addition
    // --------------------

    // fails.
    fun overflow_i8_add_incorrect(x: i8, y: i8): i8 {
        x + y
    }
    spec overflow_i8_add_incorrect {
        aborts_if false;
    }

    // succeeds.
    fun overflow_i8_add(x: i8, y: i8): i8 {
        x + y
    }
    spec overflow_i8_add {
        aborts_if x + y > MAX_I8 || x + y < MIN_I8;
    }

    // fails.
    fun overflow_i16_add_incorrect(x: i16, y: i16): i16 {
        x + y
    }
    spec overflow_i16_add_incorrect {
        aborts_if false;
    }

    // succeeds.
    fun overflow_i16_add(x: i16, y: i16): i16 {
        x + y
    }
    spec overflow_i16_add {
        aborts_if x + y > MAX_I16 || x + y < MIN_I16;
    }

    // fails.
    fun overflow_i32_add_incorrect(x: i32, y: i32): i32 {
        x + y
    }
    spec overflow_i32_add_incorrect {
        aborts_if false;
    }

    // succeeds.
    fun overflow_i32_add(x: i32, y: i32): i32 {
        x + y
    }
    spec overflow_i32_add {
        aborts_if x + y > MAX_I32 || x + y < MIN_I32;
    }

    // fails.
    fun overflow_i64_add_incorrect(x: i64, y: i64): i64 {
        x + y
    }
    spec overflow_i64_add_incorrect {
        aborts_if false;
    }

    // succeeds.
    fun overflow_i64_add(x: i64, y: i64): i64 {
        x + y
    }
    spec overflow_i64_add {
        aborts_if x + y > MAX_I64 || x + y < MIN_I64;
    }

    // fails.
    fun overflow_i128_add_incorrect(x: i128, y: i128): i128 {
        x + y
    }
    spec overflow_i128_add_incorrect {
        aborts_if false;
    }

    // succeeds.
    fun overflow_i128_add(x: i128, y: i128): i128 {
        x + y
    }
    spec overflow_i128_add {
        aborts_if x + y > MAX_I128 || x + y < MIN_I128;
    }

    // fails.
    fun overflow_i256_add_incorrect(x: i256, y: i256): i256 {
        x + y
    }
    spec overflow_i256_add_incorrect {
        aborts_if false;
    }

    // succeeds.
    fun overflow_i256_add(x: i256, y: i256): i256 {
        x + y
    }
    spec overflow_i256_add {
        aborts_if x + y > MAX_I256 || x + y < MIN_I256;
    }


    // --------------------------
    // Overflow by multiplication
    // --------------------------

    // fails.
    fun overflow_i8_mul_incorrect(x: i8, y: i8): i8 {
        x * y
    }
    spec overflow_i8_mul_incorrect {
        aborts_if false;
    }

    // succeeds.
    fun overflow_i8_mul(x: i8, y: i8): i8 {
        x * y
    }
    spec overflow_i8_mul {
        aborts_if x * y > MAX_I8 || x * y < MIN_I8;
    }

    // fails.
    fun overflow_i16_mul_incorrect(x: i16, y: i16): i16 {
        x * y
    }
    spec overflow_i16_mul_incorrect {
        aborts_if false;
    }

    // succeeds.
    fun overflow_i16_mul(x: i16, y: i16): i16 {
        x * y
    }
    spec overflow_i16_mul {
        aborts_if x * y > MAX_I16 || x * y < MIN_I16;
    }

    // fails.
    fun overflow_i32_mul_incorrect(x: i32, y: i32): i32 {
        x * y
    }
    spec overflow_i32_mul_incorrect {
        aborts_if false;
    }

    // succeeds.
    fun overflow_i32_mul(x: i32, y: i32): i32 {
        x * y
    }
    spec overflow_i32_mul {
        aborts_if x * y > MAX_I32 || x * y < MIN_I32;
    }

    // fails.
    fun overflow_i64_mul_incorrect(x: i64, y: i64): i64 {
        x * y
    }
    spec overflow_i64_mul_incorrect {
        aborts_if false;
    }

    fun overflow_i64_mul(x: i64, y: i64): i64 {
        x * y
    }
    spec overflow_i64_mul {
        aborts_if x * y > MAX_I64 || x * y < MIN_I64;
    }

    // fails.
    fun overflow_i128_mul_incorrect(x: i128, y: i128): i128 {
        x * y
    }
    spec overflow_i128_mul_incorrect {
        aborts_if false;
    }

    fun overflow_i128_mul(x: i128, y: i128): i128 {
        x * y
    }
    spec overflow_i128_mul {
        aborts_if x * y > MAX_I128 || x * y < MIN_I128;
    }

    // fails.
    fun overflow_i256_mul_incorrect(x: i256, y: i256): i256 {
        x * y
    }
    spec overflow_i256_mul_incorrect {
        aborts_if false;
    }

    fun overflow_i256_mul(x: i256, y: i256): i256 {
        x * y
    }
    spec overflow_i256_mul {
        aborts_if x * y > MAX_I256 || x * y < MIN_I256;
    }

}
