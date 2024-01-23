module Symbols::M3 {
    fun create_u128(a: u128, b: u128):u128 {
        a + b
    }

    fun test_may_overflow(var_u64: u64, var_u128:u128, var_u256:u256): u64{
        let unused_local_val = 1;
        
        let var_local_u64 = var_u64 + 1;
        let var_local_u128 = var_u128 * 2;
        let var_local_u256 = var_u256 << 3;
        var_local_u256 = ((((var_local_u64 as u128) + var_local_u128) as u256) * var_local_u256) << 3;
        
        var_local_u64
    }

    fun test_may_underflow(var_u64: u64, var_u128:u128, var_u256:u256): u64{
        let unused_local_val = 1;
        
        let var_local_u64 = var_u64 - 1;
        let var_local_u128 = (var_u128 * 2) - 1000;
        assert!(var_local_u128 > 0, 0);
        
        var_local_u64
    }

    fun test_may_div_zero(var_u64: u64, var_u128:u128, var_u256:u256): u64{
        let unused_local_val = 1;
        
        let var_local_u64 = (100 + var_u64) / var_u64 ;
        let var_local_u128 = var_u256 % ((var_u128 as u256) / var_u256);
        
        var_local_u64
    }

}