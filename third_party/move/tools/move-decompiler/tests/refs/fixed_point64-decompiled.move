module 0x1::fixed_point64 {
    struct FixedPoint64 has copy, drop, store {
        value: u128,
    }
    
    public fun add(arg0: FixedPoint64, arg1: FixedPoint64) : FixedPoint64 {
        let v0 = (get_raw_value(arg0) as u256) + (get_raw_value(arg1) as u256);
        assert!(v0 <= 340282366920938463463374607431768211455, 131077);
        create_from_raw_value(v0 as u128)
    }
    
    public fun almost_equal(arg0: FixedPoint64, arg1: FixedPoint64, arg2: FixedPoint64) : bool {
        let v0 = arg0.value > arg1.value;
        v0 && arg0.value - arg1.value <= arg2.value || arg1.value - arg0.value <= arg2.value
    }
    
    public fun ceil(arg0: FixedPoint64) : u128 {
        let v0 = floor(arg0) << 64;
        if (arg0.value == v0) {
            return v0 >> 64
        };
        ((v0 as u256) + 18446744073709551616 >> 64) as u128
    }
    
    public fun create_from_rational(arg0: u128, arg1: u128) : FixedPoint64 {
        assert!(arg1 != 0, 65537);
        let v0 = ((arg0 as u256) << 64) / (arg1 as u256);
        assert!(v0 != 0 || arg0 == 0, 131077);
        assert!(v0 <= 340282366920938463463374607431768211455, 131077);
        FixedPoint64{value: v0 as u128}
    }
    
    public fun create_from_raw_value(arg0: u128) : FixedPoint64 {
        FixedPoint64{value: arg0}
    }
    
    public fun create_from_u128(arg0: u128) : FixedPoint64 {
        let v0 = (arg0 as u256) << 64;
        assert!(v0 <= 340282366920938463463374607431768211455, 131077);
        FixedPoint64{value: v0 as u128}
    }
    
    public fun divide_u128(arg0: u128, arg1: FixedPoint64) : u128 {
        assert!(arg1.value != 0, 65540);
        let v0 = ((arg0 as u256) << 64) / (arg1.value as u256);
        assert!(v0 <= 340282366920938463463374607431768211455, 131074);
        v0 as u128
    }
    
    public fun equal(arg0: FixedPoint64, arg1: FixedPoint64) : bool {
        arg0.value == arg1.value
    }
    
    public fun floor(arg0: FixedPoint64) : u128 {
        arg0.value >> 64
    }
    
    public fun get_raw_value(arg0: FixedPoint64) : u128 {
        arg0.value
    }
    
    public fun greater(arg0: FixedPoint64, arg1: FixedPoint64) : bool {
        arg0.value > arg1.value
    }
    
    public fun greater_or_equal(arg0: FixedPoint64, arg1: FixedPoint64) : bool {
        arg0.value >= arg1.value
    }
    
    public fun is_zero(arg0: FixedPoint64) : bool {
        arg0.value == 0
    }
    
    public fun less(arg0: FixedPoint64, arg1: FixedPoint64) : bool {
        arg0.value < arg1.value
    }
    
    public fun less_or_equal(arg0: FixedPoint64, arg1: FixedPoint64) : bool {
        arg0.value <= arg1.value
    }
    
    public fun max(arg0: FixedPoint64, arg1: FixedPoint64) : FixedPoint64 {
        if (arg0.value > arg1.value) {
            arg0
        } else {
            arg1
        }
    }
    
    public fun min(arg0: FixedPoint64, arg1: FixedPoint64) : FixedPoint64 {
        if (arg0.value < arg1.value) {
            arg0
        } else {
            arg1
        }
    }
    
    public fun multiply_u128(arg0: u128, arg1: FixedPoint64) : u128 {
        let v0 = (arg0 as u256) * (arg1.value as u256) >> 64;
        assert!(v0 <= 340282366920938463463374607431768211455, 131075);
        v0 as u128
    }
    
    public fun round(arg0: FixedPoint64) : u128 {
        let v0 = floor(arg0) << 64;
        if (arg0.value < v0 + 9223372036854775808) {
            v0 >> 64
        } else {
            ceil(arg0)
        }
    }
    
    public fun sub(arg0: FixedPoint64, arg1: FixedPoint64) : FixedPoint64 {
        let v0 = get_raw_value(arg0);
        let v1 = get_raw_value(arg1);
        assert!(v0 >= v1, 65542);
        create_from_raw_value(v0 - v1)
    }
    
    // decompiled from Move bytecode v6
}
