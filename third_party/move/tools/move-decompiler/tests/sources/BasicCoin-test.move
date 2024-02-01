/// This module defines a minimal and generic Coin and Balance.
module NamedAddr::BasicCoin {
    use std::signer;

    /// Error codes
    const ENOT_MODULE_OWNER: u64 = 0;
    const EINSUFFICIENT_BALANCE: u64 = 1;
    const EALREADY_HAS_BALANCE: u64 = 2;

    struct Coin<phantom CoinType> has store {
        value: u64
    }

    struct Coin2<phantom CoinType> has store {
        value: u64,
        value2: u64
    }

    struct Balance<phantom CoinType> has key {
        coin: Coin<CoinType>
    }

    /// Publish an empty balance resource under `account`'s address. This function must be called before
    /// minting or transferring to the account.
    public fun publish_balance<CoinType>(account: &signer) {
        let empty_coin = Coin<CoinType> { value: 0 };
        assert!(!exists<Balance<CoinType>>(signer::address_of(account)), EALREADY_HAS_BALANCE);
        move_to(account, Balance<CoinType> { coin: empty_coin });
    }

    /// Mint `amount` tokens to `mint_addr`. This method requires a witness with `CoinType` so that the
    /// module that owns `CoinType` can decide the minting policy.
    public fun mint<CoinType: drop>(mint_addr: address, amount: u64, _witness: CoinType) acquires Balance {
        // Deposit `total_value` amount of tokens to mint_addr's balance
        deposit(mint_addr, Coin<CoinType> { value: amount });
    }

    public fun balance_of<CoinType>(owner: address): u64 acquires Balance {
        borrow_global<Balance<CoinType>>(owner).coin.value
    }

    spec balance_of {
        pragma aborts_if_is_strict;
    }

    /// Transfers `amount` of tokens from `from` to `to`. This method requires a witness with `CoinType` so that the
    /// module that owns `CoinType` can  decide the transferring policy.
    public fun transfer<CoinType: drop>(from: &signer, to: address, amount: u64, _witness: CoinType) acquires Balance {
        let check = withdraw<CoinType>(signer::address_of(from), amount);
        deposit<CoinType>(to, check);
    }

    fun withdraw<CoinType>(addr: address, amount: u64) : Coin<CoinType> acquires Balance {
        let balance = balance_of<CoinType>(addr);
        assert!(balance >= amount, EINSUFFICIENT_BALANCE);
        let balance_ref = &mut borrow_global_mut<Balance<CoinType>>(addr).coin.value;
        *balance_ref = balance - amount;
        Coin<CoinType> { value: amount }
    }

    fun deposit<CoinType>(addr: address, check: Coin<CoinType>) acquires Balance{
        let balance = balance_of<CoinType>(addr);
        let balance_ref = &mut borrow_global_mut<Balance<CoinType>>(addr).coin.value;
        let Coin { value } = check;
        *balance_ref = balance + value;
    }

    fun test_destruct<CoinType>(addr: address, check: Coin2<CoinType>): u64 acquires Balance{
        let balance = balance_of<CoinType>(addr);
        let balance_ref = &mut borrow_global_mut<Balance<CoinType>>(addr).coin.value;
        let Coin2 { value: value_renamed, value2: value2_renamed } = &check;
        let Coin2 { value: value_renamed2, value2: value2_renamed2 } = &check;
        if (*value_renamed + *value_renamed2 == *value2_renamed - *value2_renamed2) {
            *balance_ref = balance + *value_renamed;
        };
        let Coin2 {value: final_val, value2: final_val2} = check;
        final_val + final_val2
    }

    use std::vector;

    public fun test_vector(x: u64): u64 {
        let r = 0;
        let v = vector[1,2,3,4,5,6,7,8,9];
        while (!vector::is_empty(&v)) {
            let y = vector::pop_back(&mut v);
            r = r + y * x;
        };
        r
    }

    fun pop_smallest_while_not_equal(
        v1: vector<u64>,
        v2: vector<u64>,
    ): vector<u64> {
        let result = vector::empty();
        while (!vector::is_empty(&v1) && !vector::is_empty(&v2)) {
            let u1 = *vector::borrow(&v1, vector::length(&v1) - 1);
            let u2 = *vector::borrow(&v2, vector::length(&v2) - 1);
            let popped =
                if (u1 < u2) vector::pop_back(&mut v1)
                else if (u2 < u1) vector::pop_back(&mut v2)
                else break; // Here, `break` has type `u64`
            vector::push_back(&mut result, popped);
        };

        result
    }

    fun test_ref_mut(a: u8): u8 {
        let c = &mut a;
        let b = 4;
        *c = 3;
        if (c == &b) {
            *c = 4;
        };
        a
    }

    fun test_ref(a: &u8, b: &u8): u8 {
        let c = if (*a > *b) {
            *a - *b
        } else {
            return *b;
            *b - *a
        };
        if (c > 10) {
            c = 0 - c;
        };
        c
    }

    fun test_if(a: u8, b: u8): u8 {
        let c = if (a > b) {
            a - b
        } else {
            return b;
            b - a
        };
        if (c > 10) {
            c = 0 - c;
        };
        c
    }

    fun test_while(a: u8, b: u8): u8 {
        while (a < b) {
            if (a==9) {
                return b
            };
            if (a==7) {
                break
            };
            let c = if ((a > b) && (a-b*2)/(b-a*3) < a+b) {
                a - b
            } else {
                b - a
            };
            if (a==8) {
                continue
            };
            while (c > 10) {
                c = c - 1;
                if (c % 2 == 3) {
                    break
                };
            };
            a = a + 2;
            if (c == 0-12) {
                return c-a
            };
        };
        while (a < b) {};
        77
    }

    public fun test_swap(a: u8, b: u8): u8 {
        if (a>b) {
            (a,b) = (b,a);
        };
        b-a
    }

    fun test_ints(a: u8): u32 {
        let x: u16 = (a as u16)+1;
        let y: u32 = (x as u32)+2;
        y+3
    }

    struct R has copy, drop {
        x: u64
    }

    fun test1(r_ref: &R) : u64 {
        let x_ref = & r_ref.x;
        *x_ref
    }
}
