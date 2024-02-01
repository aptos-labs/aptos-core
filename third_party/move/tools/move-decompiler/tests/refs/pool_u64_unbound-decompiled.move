module 0x1::pool_u64_unbound {
    struct Pool has store {
        total_coins: u64,
        total_shares: u128,
        shares: 0x1::table_with_length::TableWithLength<address, u128>,
        scaling_factor: u64,
    }
    
    public fun contains(arg0: &Pool, arg1: address) : bool {
        0x1::table_with_length::contains<address, u128>(&arg0.shares, arg1)
    }
    
    public fun destroy_empty(arg0: Pool) {
        assert!(arg0.total_coins == 0, 0x1::error::invalid_state(3));
        let Pool {
            total_coins    : _,
            total_shares   : _,
            shares         : v2,
            scaling_factor : _,
        } = arg0;
        0x1::table_with_length::destroy_empty<address, u128>(v2);
    }
    
    public fun new() : Pool {
        create_with_scaling_factor(1)
    }
    
    fun add_shares(arg0: &mut Pool, arg1: address, arg2: u128) : u128 {
        if (contains(arg0, arg1)) {
            let v1 = 0x1::table_with_length::borrow_mut<address, u128>(&mut arg0.shares, arg1);
            let v2 = *v1;
            assert!(340282366920938463463374607431768211455 - v2 >= arg2, 0x1::error::invalid_argument(5));
            *v1 = v2 + arg2;
            *v1
        } else {
            if (arg2 > 0) {
                0x1::table_with_length::add<address, u128>(&mut arg0.shares, arg1, arg2);
            };
            arg2
        }
    }
    
    public fun amount_to_shares(arg0: &Pool, arg1: u64) : u128 {
        amount_to_shares_with_total_coins(arg0, arg1, arg0.total_coins)
    }
    
    public fun amount_to_shares_with_total_coins(arg0: &Pool, arg1: u64, arg2: u64) : u128 {
        if (arg0.total_coins == 0 || arg0.total_shares == 0) {
            to_u128(arg1) * to_u128(arg0.scaling_factor)
        } else {
            multiply_then_divide(arg0, to_u128(arg1), arg0.total_shares, to_u128(arg2))
        }
    }
    
    public fun balance(arg0: &Pool, arg1: address) : u64 {
        shares_to_amount(arg0, shares(arg0, arg1))
    }
    
    public fun buy_in(arg0: &mut Pool, arg1: address, arg2: u64) : u128 {
        if (arg2 == 0) {
            return 0
        };
        let v0 = amount_to_shares(arg0, arg2);
        assert!(18446744073709551615 - arg0.total_coins >= arg2, 0x1::error::invalid_argument(6));
        let v1 = 340282366920938463463374607431768211455 - arg0.total_shares >= v0;
        assert!(v1, 0x1::error::invalid_argument(7));
        arg0.total_coins = arg0.total_coins + arg2;
        arg0.total_shares = arg0.total_shares + v0;
        add_shares(arg0, arg1, v0);
        v0
    }
    
    public fun create() : Pool {
        new()
    }
    
    public fun create_with_scaling_factor(arg0: u64) : Pool {
        let v0 = 0x1::table_with_length::new<address, u128>();
        Pool{
            total_coins    : 0, 
            total_shares   : 0, 
            shares         : v0, 
            scaling_factor : arg0,
        }
    }
    
    fun deduct_shares(arg0: &mut Pool, arg1: address, arg2: u128) : u128 {
        assert!(contains(arg0, arg1), 0x1::error::invalid_argument(1));
        assert!(shares(arg0, arg1) >= arg2, 0x1::error::invalid_argument(4));
        let v0 = 0x1::table_with_length::borrow_mut<address, u128>(&mut arg0.shares, arg1);
        *v0 = *v0 - arg2;
        let v1 = *v0;
        if (v1 == 0) {
            0x1::table_with_length::remove<address, u128>(&mut arg0.shares, arg1);
        };
        v1
    }
    
    public fun multiply_then_divide(arg0: &Pool, arg1: u128, arg2: u128, arg3: u128) : u128 {
        (to_u256(arg1) * to_u256(arg2) / to_u256(arg3)) as u128
    }
    
    public fun redeem_shares(arg0: &mut Pool, arg1: address, arg2: u128) : u64 {
        assert!(contains(arg0, arg1), 0x1::error::invalid_argument(1));
        assert!(shares(arg0, arg1) >= arg2, 0x1::error::invalid_argument(4));
        if (arg2 == 0) {
            return 0
        };
        let v0 = shares_to_amount(arg0, arg2);
        arg0.total_coins = arg0.total_coins - v0;
        arg0.total_shares = arg0.total_shares - arg2;
        deduct_shares(arg0, arg1, arg2);
        v0
    }
    
    public fun shareholders_count(arg0: &Pool) : u64 {
        0x1::table_with_length::length<address, u128>(&arg0.shares)
    }
    
    public fun shares(arg0: &Pool, arg1: address) : u128 {
        if (contains(arg0, arg1)) {
            *0x1::table_with_length::borrow<address, u128>(&arg0.shares, arg1)
        } else {
            0
        }
    }
    
    public fun shares_to_amount(arg0: &Pool, arg1: u128) : u64 {
        shares_to_amount_with_total_coins(arg0, arg1, arg0.total_coins)
    }
    
    public fun shares_to_amount_with_total_coins(arg0: &Pool, arg1: u128, arg2: u64) : u64 {
        if (arg0.total_coins == 0 || arg0.total_shares == 0) {
            0
        } else {
            multiply_then_divide(arg0, arg1, to_u128(arg2), arg0.total_shares) as u64
        }
    }
    
    public fun shares_to_amount_with_total_stats(arg0: &Pool, arg1: u128, arg2: u64, arg3: u128) : u64 {
        if (arg0.total_coins == 0 || arg3 == 0) {
            0
        } else {
            multiply_then_divide(arg0, arg1, to_u128(arg2), arg3) as u64
        }
    }
    
    fun to_u128(arg0: u64) : u128 {
        arg0 as u128
    }
    
    fun to_u256(arg0: u128) : u256 {
        arg0 as u256
    }
    
    public fun total_coins(arg0: &Pool) : u64 {
        arg0.total_coins
    }
    
    public fun total_shares(arg0: &Pool) : u128 {
        arg0.total_shares
    }
    
    public fun transfer_shares(arg0: &mut Pool, arg1: address, arg2: address, arg3: u128) {
        assert!(contains(arg0, arg1), 0x1::error::invalid_argument(1));
        assert!(shares(arg0, arg1) >= arg3, 0x1::error::invalid_argument(4));
        if (arg3 == 0) {
            return
        };
        deduct_shares(arg0, arg1, arg3);
        add_shares(arg0, arg2, arg3);
    }
    
    public fun update_total_coins(arg0: &mut Pool, arg1: u64) {
        arg0.total_coins = arg1;
    }
    
    // decompiled from Move bytecode v6
}
