module 0x1::pool_u64 {
    struct Pool has store {
        shareholders_limit: u64,
        total_coins: u64,
        total_shares: u64,
        shares: 0x1::simple_map::SimpleMap<address, u64>,
        shareholders: vector<address>,
        scaling_factor: u64,
    }
    
    public fun contains(arg0: &Pool, arg1: address) : bool {
        0x1::simple_map::contains_key<address, u64>(&arg0.shares, &arg1)
    }
    
    public fun create(arg0: u64) : Pool {
        new(arg0)
    }
    
    public fun destroy_empty(arg0: Pool) {
        assert!(arg0.total_coins == 0, 0x1::error::invalid_state(3));
        let Pool {
            shareholders_limit : _,
            total_coins        : _,
            total_shares       : _,
            shares             : _,
            shareholders       : _,
            scaling_factor     : _,
        } = arg0;
    }
    
    fun add_shares(arg0: &mut Pool, arg1: address, arg2: u64) : u64 {
        if (contains(arg0, arg1)) {
            let v1 = 0x1::simple_map::borrow_mut<address, u64>(&mut arg0.shares, &arg1);
            let v2 = *v1;
            assert!(18446744073709551615 - v2 >= arg2, 0x1::error::invalid_argument(5));
            *v1 = v2 + arg2;
            *v1
        } else {
            if (arg2 > 0) {
                let v3 = 0x1::vector::length<address>(&arg0.shareholders) < arg0.shareholders_limit;
                assert!(v3, 0x1::error::invalid_state(2));
                0x1::vector::push_back<address>(&mut arg0.shareholders, arg1);
                0x1::simple_map::add<address, u64>(&mut arg0.shares, arg1, arg2);
            };
            arg2
        }
    }
    
    public fun amount_to_shares(arg0: &Pool, arg1: u64) : u64 {
        amount_to_shares_with_total_coins(arg0, arg1, arg0.total_coins)
    }
    
    public fun amount_to_shares_with_total_coins(arg0: &Pool, arg1: u64, arg2: u64) : u64 {
        if (arg0.total_coins == 0 || arg0.total_shares == 0) {
            arg1 * arg0.scaling_factor
        } else {
            multiply_then_divide(arg0, arg1, arg0.total_shares, arg2)
        }
    }
    
    public fun balance(arg0: &Pool, arg1: address) : u64 {
        shares_to_amount(arg0, shares(arg0, arg1))
    }
    
    public fun buy_in(arg0: &mut Pool, arg1: address, arg2: u64) : u64 {
        if (arg2 == 0) {
            return 0
        };
        let v0 = amount_to_shares(arg0, arg2);
        assert!(18446744073709551615 - arg0.total_coins >= arg2, 0x1::error::invalid_argument(6));
        assert!(18446744073709551615 - arg0.total_shares >= v0, 0x1::error::invalid_argument(6));
        arg0.total_coins = arg0.total_coins + arg2;
        arg0.total_shares = arg0.total_shares + v0;
        add_shares(arg0, arg1, v0);
        v0
    }
    
    public fun create_with_scaling_factor(arg0: u64, arg1: u64) : Pool {
        let v0 = 0x1::simple_map::create<address, u64>();
        let v1 = 0x1::vector::empty<address>();
        Pool{
            shareholders_limit : arg0, 
            total_coins        : 0, 
            total_shares       : 0, 
            shares             : v0, 
            shareholders       : v1, 
            scaling_factor     : arg1,
        }
    }
    
    fun deduct_shares(arg0: &mut Pool, arg1: address, arg2: u64) : u64 {
        assert!(contains(arg0, arg1), 0x1::error::invalid_argument(1));
        assert!(shares(arg0, arg1) >= arg2, 0x1::error::invalid_argument(4));
        let v0 = 0x1::simple_map::borrow_mut<address, u64>(&mut arg0.shares, &arg1);
        *v0 = *v0 - arg2;
        let v1 = *v0;
        if (v1 == 0) {
            let (_, v3) = 0x1::vector::index_of<address>(&arg0.shareholders, &arg1);
            0x1::vector::remove<address>(&mut arg0.shareholders, v3);
            let (_, _) = 0x1::simple_map::remove<address, u64>(&mut arg0.shares, &arg1);
        };
        v1
    }
    
    public fun multiply_then_divide(arg0: &Pool, arg1: u64, arg2: u64, arg3: u64) : u64 {
        (to_u128(arg1) * to_u128(arg2) / to_u128(arg3)) as u64
    }
    
    public fun new(arg0: u64) : Pool {
        create_with_scaling_factor(arg0, 1)
    }
    
    public fun redeem_shares(arg0: &mut Pool, arg1: address, arg2: u64) : u64 {
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
    
    public fun shareholders(arg0: &Pool) : vector<address> {
        arg0.shareholders
    }
    
    public fun shareholders_count(arg0: &Pool) : u64 {
        0x1::vector::length<address>(&arg0.shareholders)
    }
    
    public fun shares(arg0: &Pool, arg1: address) : u64 {
        if (contains(arg0, arg1)) {
            *0x1::simple_map::borrow<address, u64>(&arg0.shares, &arg1)
        } else {
            0
        }
    }
    
    public fun shares_to_amount(arg0: &Pool, arg1: u64) : u64 {
        shares_to_amount_with_total_coins(arg0, arg1, arg0.total_coins)
    }
    
    public fun shares_to_amount_with_total_coins(arg0: &Pool, arg1: u64, arg2: u64) : u64 {
        if (arg0.total_coins == 0 || arg0.total_shares == 0) {
            0
        } else {
            multiply_then_divide(arg0, arg1, arg2, arg0.total_shares)
        }
    }
    
    fun to_u128(arg0: u64) : u128 {
        arg0 as u128
    }
    
    public fun total_coins(arg0: &Pool) : u64 {
        arg0.total_coins
    }
    
    public fun total_shares(arg0: &Pool) : u64 {
        arg0.total_shares
    }
    
    public fun transfer_shares(arg0: &mut Pool, arg1: address, arg2: address, arg3: u64) {
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
