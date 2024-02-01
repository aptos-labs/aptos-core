module 0x1::math_fixed64 {
    public fun sqrt(arg0: 0x1::fixed_point64::FixedPoint64) : 0x1::fixed_point64::FixedPoint64 {
        let v0 = 0x1::fixed_point64::get_raw_value(arg0);
        let v1 = (0x1::math128::sqrt(v0) << 32) as u256;
        0x1::fixed_point64::create_from_raw_value((v1 + ((v0 as u256) << 64) / v1 >> 1) as u128)
    }
    
    public fun exp(arg0: 0x1::fixed_point64::FixedPoint64) : 0x1::fixed_point64::FixedPoint64 {
        let v0 = exp_raw(0x1::fixed_point64::get_raw_value(arg0) as u256) as u128;
        0x1::fixed_point64::create_from_raw_value(v0)
    }
    
    fun exp_raw(arg0: u256) : u256 {
        let v0 = arg0 / 12786308645202655660;
        assert!(v0 <= 63, 0x1::error::invalid_state(1));
        let v1 = v0 as u8;
        let v2 = arg0 % 12786308645202655660;
        let v3 = 22045359733108027;
        let v4 = v2 / v3;
        let v5 = v2 % v3;
        let v6 = pow_raw(18468802611690918839, v4 as u128);
        let v7 = v6 - (v6 * 219071715585908898 * v4 >> 128);
        let v8 = v7 * v5 >> 64 - v1;
        let v9 = v8 * v5 >> 64;
        let v10 = v9 * v5 >> 64;
        let v11 = v10 * v5 >> 64;
        let v12 = v11 * v5 >> 64;
        (v7 << v1) + v8 + v9 / 2 + v10 / 6 + v11 / 24 + v12 / 120 + (v12 * v5 >> 64) / 720
    }
    
    public fun ln_plus_32ln2(arg0: 0x1::fixed_point64::FixedPoint64) : 0x1::fixed_point64::FixedPoint64 {
        let v0 = 0x1::fixed_point64::get_raw_value(0x1::math128::log2_64(0x1::fixed_point64::get_raw_value(arg0)));
        0x1::fixed_point64::create_from_raw_value(((v0 as u256) * 12786308645202655660 >> 64) as u128)
    }
    
    public fun log2_plus_64(arg0: 0x1::fixed_point64::FixedPoint64) : 0x1::fixed_point64::FixedPoint64 {
        0x1::math128::log2_64(0x1::fixed_point64::get_raw_value(arg0) as u128)
    }
    
    public fun mul_div(arg0: 0x1::fixed_point64::FixedPoint64, arg1: 0x1::fixed_point64::FixedPoint64, arg2: 0x1::fixed_point64::FixedPoint64) : 0x1::fixed_point64::FixedPoint64 {
        let v0 = 0x1::fixed_point64::get_raw_value(arg1);
        let v1 = 0x1::fixed_point64::get_raw_value(arg2);
        assert!(v1 != 0, 0x1::error::invalid_argument(4));
        let v2 = ((0x1::fixed_point64::get_raw_value(arg0) as u256) * (v0 as u256) / (v1 as u256)) as u128;
        0x1::fixed_point64::create_from_raw_value(v2)
    }
    
    public fun pow(arg0: 0x1::fixed_point64::FixedPoint64, arg1: u64) : 0x1::fixed_point64::FixedPoint64 {
        let v0 = pow_raw(0x1::fixed_point64::get_raw_value(arg0) as u256, arg1 as u128) as u128;
        0x1::fixed_point64::create_from_raw_value(v0)
    }
    
    fun pow_raw(arg0: u256, arg1: u128) : u256 {
        let v0 = 18446744073709551616;
        while (arg1 != 0) {
            if (arg1 & 1 != 0) {
                let v1 = v0 * arg0;
                v0 = v1 >> 64;
            };
            arg1 = arg1 >> 1;
            let v2 = arg0 * arg0;
            arg0 = v2 >> 64;
        };
        v0
    }
    
    // decompiled from Move bytecode v6
}
