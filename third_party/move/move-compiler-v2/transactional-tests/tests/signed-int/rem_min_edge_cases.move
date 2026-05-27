//# publish
module 0xff::rem_min {
    public fun min_i8(): i8 { -128i8 }
    public fun min_i16(): i16 { -32768i16 }
    public fun min_i32(): i32 { -2147483648i32 }
    public fun min_i64(): i64 { -9223372036854775808i64 }
    public fun min_i128(): i128 {
        -170141183460469231731687303715884105728i128
    }
    public fun min_i256(): i256 {
        -57896044618658097711785492504343953926634992332820282019728792003956564819968i256
    }

    public fun min_i8_rem_neg1(): i8     { min_i8()   % -1i8   }
    public fun min_i16_rem_neg1(): i16   { min_i16()  % -1i16  }
    public fun min_i32_rem_neg1(): i32   { min_i32()  % -1i32  }
    public fun min_i64_rem_neg1(): i64   { min_i64()  % -1i64  }
    public fun min_i128_rem_neg1(): i128 { min_i128() % -1i128 }
    public fun min_i256_rem_neg1(): i256 { min_i256() % -1i256 }

    public fun min_i8_rem_0(): i8     { min_i8()   % 0i8   }
    public fun min_i16_rem_0(): i16   { min_i16()  % 0i16  }
    public fun min_i32_rem_0(): i32   { min_i32()  % 0i32  }
    public fun min_i64_rem_0(): i64   { min_i64()  % 0i64  }
    public fun min_i128_rem_0(): i128 { min_i128() % 0i128 }
    public fun min_i256_rem_0(): i256 { min_i256() % 0i256 }
}

//# publish
module 0xff::const_rem_neg1 {
    const C_I8:   i8   = -128i8 % -1i8;
    const C_I16:  i16  = -32768i16 % -1i16;
    const C_I32:  i32  = -2147483648i32 % -1i32;
    const C_I64:  i64  = -9223372036854775808i64 % -1i64;
    const C_I128: i128 =
        -170141183460469231731687303715884105728i128 % -1i128;
    const C_I256: i256 =
        -57896044618658097711785492504343953926634992332820282019728792003956564819968i256
            % -1i256;

    public fun get_i8():   i8   { C_I8   }
    public fun get_i16():  i16  { C_I16  }
    public fun get_i32():  i32  { C_I32  }
    public fun get_i64():  i64  { C_I64  }
    public fun get_i128(): i128 { C_I128 }
    public fun get_i256(): i256 { C_I256 }
}

//# publish
module 0xff::const_rem_0 {
    const C_I8:   i8   = -128i8 % 0i8;
    const C_I16:  i16  = -32768i16 % 0i16;
    const C_I32:  i32  = -2147483648i32 % 0i32;
    const C_I64:  i64  = -9223372036854775808i64 % 0i64;
    const C_I128: i128 =
        -170141183460469231731687303715884105728i128 % 0i128;
    const C_I256: i256 =
        -57896044618658097711785492504343953926634992332820282019728792003956564819968i256
            % 0i256;

    public fun get_i8():   i8   { C_I8   }
    public fun get_i16():  i16  { C_I16  }
    public fun get_i32():  i32  { C_I32  }
    public fun get_i64():  i64  { C_I64  }
    public fun get_i128(): i128 { C_I128 }
    public fun get_i256(): i256 { C_I256 }
}

//# run --verbose
script {
    fun main() {
        assert!(-128i8 % -1i8 == 0i8, 100);
    }
}

//# run --verbose
script {
    fun main() {
        assert!(-32768i16 % -1i16 == 0i16, 200);
    }
}

//# run --verbose
script {
    fun main() {
        assert!(-2147483648i32 % -1i32 == 0i32, 300);
    }
}

//# run --verbose
script {
    fun main() {
        assert!(-9223372036854775808i64 % -1i64 == 0i64, 400);
    }
}

//# run --verbose
script {
    fun main() {
        assert!(
            -170141183460469231731687303715884105728i128 % -1i128 == 0i128,
            500,
        );
    }
}

//# run --verbose
script {
    fun main() {
        assert!(
            -57896044618658097711785492504343953926634992332820282019728792003956564819968i256
                % -1i256 == 0i256,
            600,
        );
    }
}


//# run --verbose
script {
    fun main() {
        -128i8 % 0i8;
    }
}

//# run --verbose
script {
    fun main() {
        -32768i16 % 0i16;
    }
}

//# run --verbose
script {
    fun main() {
        -2147483648i32 % 0i32;
    }
}

//# run --verbose
script {
    fun main() {
        -9223372036854775808i64 % 0i64;
    }
}

//# run --verbose
script {
    fun main() {
        -170141183460469231731687303715884105728i128 % 0i128;
    }
}

//# run --verbose
script {
    fun main() {
        -57896044618658097711785492504343953926634992332820282019728792003956564819968i256
            % 0i256;
    }
}


//# run 0xff::rem_min::min_i8_rem_0 --verbose

//# run 0xff::rem_min::min_i16_rem_0 --verbose

//# run 0xff::rem_min::min_i32_rem_0 --verbose

//# run 0xff::rem_min::min_i64_rem_0 --verbose

//# run 0xff::rem_min::min_i128_rem_0 --verbose

//# run 0xff::rem_min::min_i256_rem_0 --verbose
