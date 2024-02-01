module 0x1::math_fixed {
    public fun sqrt(arg0: 0x1::fixed_point32::FixedPoint32) : 0x1::fixed_point32::FixedPoint32 {
        let v0 = 0x1::math128::sqrt((0x1::fixed_point32::get_raw_value(arg0) as u128) << 32) as u64;
        0x1::fixed_point32::create_from_raw_value(v0)
    }
    
    public fun exp(arg0: 0x1::fixed_point32::FixedPoint32) : 0x1::fixed_point32::FixedPoint32 {
        let v0 = exp_raw(0x1::fixed_point32::get_raw_value(arg0) as u128) as u64;
        0x1::fixed_point32::create_from_raw_value(v0)
    }
    
    fun exp_raw(arg0: u128) : u128 {
        let v0 = arg0 / 2977044472;
        assert!(v0 <= 31, 0x1::error::invalid_state(1));
        let v1 = v0 as u8;
        let v2 = arg0 % 2977044472;
        let v3 = 595528;
        let v4 = v2 / v3;
        let v5 = v2 % v3;
        let v6 = pow_raw(4295562865, v4);
        let v7 = v6 + (v6 * 1241009291 * v4 >> 64);
        let v8 = v7 * v5 >> 32 - v1;
        let v9 = v8 * v5 >> 32;
        (v7 << v1) + v8 + v9 / 2 + (v9 * v5 >> 32) / 6
    }
    
    public fun ln_plus_32ln2(arg0: 0x1::fixed_point32::FixedPoint32) : 0x1::fixed_point32::FixedPoint32 {
        let v0 = 0x1::fixed_point32::get_raw_value(arg0) as u128;
        let v1 = ((0x1::fixed_point32::get_raw_value(0x1::math128::log2(v0)) as u128) * 2977044472 >> 32) as u64;
        0x1::fixed_point32::create_from_raw_value(v1)
    }
    
    public fun log2_plus_32(arg0: 0x1::fixed_point32::FixedPoint32) : 0x1::fixed_point32::FixedPoint32 {
        0x1::math128::log2(0x1::fixed_point32::get_raw_value(arg0) as u128)
    }
    
    public fun mul_div(arg0: 0x1::fixed_point32::FixedPoint32, arg1: 0x1::fixed_point32::FixedPoint32, arg2: 0x1::fixed_point32::FixedPoint32) : 0x1::fixed_point32::FixedPoint32 {
        let v0 = 0x1::fixed_point32::get_raw_value(arg1);
        let v1 = 0x1::fixed_point32::get_raw_value(arg2);
        assert!(v1 != 0, 0x1::error::invalid_argument(4));
        let v2 = ((0x1::fixed_point32::get_raw_value(arg0) as u128) * (v0 as u128) / (v1 as u128)) as u64;
        0x1::fixed_point32::create_from_raw_value(v2)
    }
    
    public fun pow(arg0: 0x1::fixed_point32::FixedPoint32, arg1: u64) : 0x1::fixed_point32::FixedPoint32 {
        let v0 = pow_raw(0x1::fixed_point32::get_raw_value(arg0) as u128, arg1 as u128) as u64;
        0x1::fixed_point32::create_from_raw_value(v0)
    }
    
    fun pow_raw(arg0: u128, arg1: u128) : u128 {
        let v0 = 18446744073709551616;
        arg0 = arg0 << 32;
        while (arg1 != 0) {
            if (arg1 & 1 != 0) {
                let v1 = v0 * (arg0 as u256);
                v0 = v1 >> 64;
            };
            arg1 = arg1 >> 1;
            let v2 = (arg0 as u256) * (arg0 as u256) >> 64;
            arg0 = v2 as u128;
        };
        (v0 >> 32) as u128
    }
    
    // decompiled from Move bytecode v6
}
