module 0xbadbadbad::BasicCoin {
    struct Balance<phantom T0> has key {
        coin: Coin<T0>,
    }
    
    struct Coin<phantom T0> has store {
        value: u64,
    }
    
    struct Coin2<phantom T0> has store {
        value: u64,
        value2: u64,
    }
    
    struct R has copy, drop {
        x: u64,
    }
    
    public fun balance_of<T0>(arg0: address) : u64 acquires Balance {
        borrow_global<Balance<T0>>(arg0).coin.value
    }
    
    fun deposit<T0>(arg0: address, arg1: Coin<T0>) acquires Balance {
        let v0 = balance_of<T0>(arg0);
        let Coin { value: v1 } = arg1;
        borrow_global_mut<Balance<T0>>(arg0).coin.value = v0 + v1;
    }
    
    public fun mint<T0: drop>(arg0: address, arg1: u64, arg2: T0) acquires Balance {
        let v0 = Coin<T0>{value: arg1};
        deposit<T0>(arg0, v0);
    }
    
    fun pop_smallest_while_not_equal(arg0: vector<u64>, arg1: vector<u64>) : vector<u64> {
        let v0 = 0x1::vector::empty<u64>();
        while (!0x1::vector::is_empty<u64>(&arg0) && !0x1::vector::is_empty<u64>(&arg1)) {
            let v1 = *0x1::vector::borrow<u64>(&arg0, 0x1::vector::length<u64>(&arg0) - 1);
            let v2 = *0x1::vector::borrow<u64>(&arg1, 0x1::vector::length<u64>(&arg1) - 1);
            let v3 = if (v1 < v2) {
                0x1::vector::pop_back<u64>(&mut arg0)
            } else {
                if (v2 < v1) {
                    0x1::vector::pop_back<u64>(&mut arg1)
                } else {
                    break
                }
            };
            0x1::vector::push_back<u64>(&mut v0, v3);
        };
        v0
    }
    
    public fun publish_balance<T0>(arg0: &signer) {
        assert!(!exists<Balance<T0>>(0x1::signer::address_of(arg0)), 2);
        let v0 = Coin<T0>{value: 0};
        let v1 = Balance<T0>{coin: v0};
        move_to<Balance<T0>>(arg0, v1);
    }
    
    fun test1(arg0: &R) : u64 {
        arg0.x
    }
    
    fun test_destruct<T0>(arg0: address, arg1: Coin2<T0>) : u64 acquires Balance {
        let v0 = balance_of<T0>(arg0);
        let v1 = &arg1;
        let v2 = &v1.value;
        let v3 = &arg1;
        if (*v2 + v3.value == v1.value2 - v3.value2) {
            borrow_global_mut<Balance<T0>>(arg0).coin.value = v0 + *v2;
        };
        let Coin2 {
            value  : v4,
            value2 : v5,
        } = arg1;
        v4 + v5
    }
    
    fun test_if(arg0: u8, arg1: u8) : u8 {
        if (arg0 > arg1) {
            let v0 = arg0 - arg1;
            let v1 = v0;
            if (v0 > 10) {
                v1 = 0 - v0;
            };
            return v1
        };
        arg1
    }
    
    fun test_ints(arg0: u8) : u32 {
        (((arg0 as u16) + 1) as u32) + 2 + 3
    }
    
    fun test_ref(arg0: &u8, arg1: &u8) : u8 {
        if (*arg0 > *arg1) {
            let v0 = *arg0 - *arg1;
            let v1 = v0;
            if (v0 > 10) {
                v1 = 0 - v0;
            };
            return v1
        };
        *arg1
    }
    
    fun test_ref_mut(arg0: u8) : u8 {
        let v0 = &mut arg0;
        let v1 = 4;
        *v0 = 3;
        if (v0 == &v1) {
            *v0 = 4;
        };
        arg0
    }
    
    public fun test_swap(arg0: u8, arg1: u8) : u8 {
        if (arg0 > arg1) {
            let v0 = arg1;
            arg1 = arg0;
            arg0 = v0;
        };
        arg1 - arg0
    }
    
    public fun test_vector(arg0: u64) : u64 {
        let v0 = 0;
        let v1 = vector[1, 2, 3, 4, 5, 6, 7, 8, 9];
        while (!0x1::vector::is_empty<u64>(&v1)) {
            v0 = v0 + 0x1::vector::pop_back<u64>(&mut v1) * arg0;
        };
        v0
    }
    
    fun test_while(arg0: u8, arg1: u8) : u8 {
        while (arg0 < arg1) {
            if (arg0 == 9) {
                return arg1
            };
            if (arg0 == 7) {
                break
            };
            let v0 = if (arg0 > arg1 && (arg0 - arg1 * 2) / (arg1 - arg0 * 3) < arg0 + arg1) {
                arg0 - arg1
            } else {
                arg1 - arg0
            };
            let v1 = v0;
            if (arg0 == 8) {
                continue
            };
            while (v1 > 10) {
                let v2 = v1 - 1;
                v1 = v2;
                if (v2 % 2 == 3) {
                    break
                };
            };
            let v3 = arg0 + 2;
            arg0 = v3;
            if (v1 == 0 - 12) {
                return v1 - v3
            };
        };
        while (arg0 < arg1) {
        };
        77
    }
    
    public fun transfer<T0: drop>(arg0: &signer, arg1: address, arg2: u64, arg3: T0) acquires Balance {
        let v0 = withdraw<T0>(0x1::signer::address_of(arg0), arg2);
        deposit<T0>(arg1, v0);
    }
    
    fun withdraw<T0>(arg0: address, arg1: u64) : Coin<T0> acquires Balance {
        let v0 = balance_of<T0>(arg0);
        assert!(v0 >= arg1, 1);
        borrow_global_mut<Balance<T0>>(arg0).coin.value = v0 - arg1;
        Coin<T0>{value: arg1}
    }
    
    // decompiled from Move bytecode v6
}
