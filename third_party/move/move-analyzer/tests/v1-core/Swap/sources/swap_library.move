module SwapDeployer::AnimeSwapPoolV1Library {
    use std::signer;
    use std::type_info;
    use aptos_std::string;
    use aptos_std::comparator::Self;
    use aptos_framework::coin;
    use std::option::{Self};

    /// Maximum of u128
    const MAX_U128: u128 = 340282366920938463463374607431768211455;

    /// When not enough amount for pool
    const ERR_INSUFFICIENT_AMOUNT: u64 = 201;
    /// When not enough liquidity amount
    const ERR_INSUFFICIENT_LIQUIDITY: u64 = 202;
    /// When not enough input amount
    const ERR_INSUFFICIENT_INPUT_AMOUNT: u64 = 203;
    /// When not enough output amount
    const ERR_INSUFFICIENT_OUTPUT_AMOUNT: u64 = 204;
    /// When two coin type is the same
    const ERR_COIN_TYPE_SAME_ERROR: u64 = 205;

    /// given some amount of an asset and pair reserves, returns an equivalent amount of the other asset
    public fun quote(
        amount_x: u64,
        reserve_x: u64,
        reserve_y: u64
    ) :u64 {
        assert!(amount_x > 0, ERR_INSUFFICIENT_AMOUNT);
        assert!(reserve_x > 0 && reserve_y > 0, ERR_INSUFFICIENT_LIQUIDITY);
        let amount_y = ((amount_x as u128) * (reserve_y as u128) / (reserve_x as u128) as u64);
        amount_y
    }

    /// given an input amount of an asset and pair reserves, returns the maximum output amount of the other asset
    public fun get_amount_out(
        amount_in: u64,
        reserve_in: u64,
        reserve_out: u64,
        swap_fee: u64
    ): u64 {
        assert!(amount_in > 0, ERR_INSUFFICIENT_INPUT_AMOUNT);
        assert!(reserve_in > 0 && reserve_out > 0, ERR_INSUFFICIENT_LIQUIDITY);
        let amount_in_with_fee = (amount_in as u128) * ((10000 - swap_fee) as u128);
        let numerator = amount_in_with_fee * (reserve_out as u128);
        let denominator = (reserve_in as u128) * 10000 + amount_in_with_fee;
        let amount_out = numerator / denominator;
        (amount_out as u64)
    }

    /// given an output amount of an asset and pair reserves, returns a required input amount of the other asset
    public fun get_amount_in(
        amount_out: u64,
        reserve_in: u64,
        reserve_out: u64,
        swap_fee: u64
    ): u64 {
        assert!(amount_out > 0, ERR_INSUFFICIENT_OUTPUT_AMOUNT);
        assert!(reserve_in > 0 && reserve_out > 0, ERR_INSUFFICIENT_LIQUIDITY);
        let numerator = (reserve_in as u128) * (amount_out as u128) * 10000;
        let denominator = ((reserve_out - amount_out) as u128) * ((10000 - swap_fee) as u128);
        let amount_in = numerator / denominator + 1;
        (amount_in as u64)
    }

    // sqrt function
    public fun sqrt(
        x: u64,
        y: u64
    ): u64 {
        sqrt_128((x as u128) * (y as u128))
    }

    /// babylonian method (https://en.wikipedia.org/wiki/Methods_of_computing_square_roots#Babylonian_method)
    public fun sqrt_128(
        y: u128
    ): u64 {
        if (y < 4) {
            if (y == 0) {
                0
            } else {
                1
            }
        } else {
            let z = y;
            let x = y / 2 + 1;
            while (x < z) {
                z = x;
                x = (y / x + x) / 2;
            };
            (z as u64)
        }
    }

    /// return Math.min
    public fun min(
        x:u64,
        y:u64
    ): u64 {
        if (x < y) return x else return y
    }

    /// Add but allow overflow
    public fun overflow_add(a: u128, b: u128): u128 {
        let r = MAX_U128 - b;
        if (r < a) {
            return a - r - 1
        };
        r = MAX_U128 - a;
        if (r < b) {
            return b - r - 1
        };
        a + b
    }

    // Check if mul maybe overflow
    // The result maybe false positive
    public fun is_overflow_mul(a: u128, b: u128): bool {
        MAX_U128 / b <= a
    }

    // compare type, when use, CoinType1 should < CoinType2
    public fun compare<CoinType1, CoinType2>(): bool{
        let type_name_coin_1 = type_info::type_name<CoinType1>();
        let type_name_coin_2 = type_info::type_name<CoinType2>();
        assert!(type_name_coin_1 != type_name_coin_2, ERR_COIN_TYPE_SAME_ERROR);

        if (string::length(&type_name_coin_1) < string::length(&type_name_coin_2)) return true;
        if (string::length(&type_name_coin_1) > string::length(&type_name_coin_2)) return false;

        let struct_cmp = comparator::compare(&type_name_coin_1, &type_name_coin_2);
        comparator::is_smaller_than(&struct_cmp)
    }

    // get coin::supply<LPCoin>
    public fun get_lpcoin_total_supply<LPCoin>(): u128 {
        option::extract(&mut coin::supply<LPCoin>())
    }

    // register coin if not registered
    public fun register_coin<CoinType>(
        account: &signer
    ) {
        let account_addr = signer::address_of(account);
        if (!coin::is_account_registered<CoinType>(account_addr)) {
            coin::register<CoinType>(account);
        };
    }

    #[test_only]
    const TEST_ERROR:u64 = 10000;
    #[test_only]
    const SQRT_ERROR:u64 = 10001;
    #[test_only]
    const QUOTE_ERROR:u64 = 10002;

    #[test]
    public entry fun test_overflow_add() {
        let u128_max_add_1_u256 = overflow_add(MAX_U128, 1);
        let u128_max_add_2_u256 = overflow_add(MAX_U128, 2);
        assert!(u128_max_add_1_u256 == 0, TEST_ERROR);
        assert!(u128_max_add_2_u256 == 1, TEST_ERROR);
    }

    #[test]
    public entry fun test_is_overflow_mul() {
        let overflow_1 = is_overflow_mul(MAX_U128 / 2, 3);
        let overflow_2 = is_overflow_mul(MAX_U128 / 3, 3);  // false positive
        let not_overflow_1 = is_overflow_mul(MAX_U128 / 2 - 1, 2);
        let not_overflow_2 = is_overflow_mul(MAX_U128 / 3 - 1, 3);
        assert!(overflow_1, TEST_ERROR);
        assert!(overflow_2, TEST_ERROR);
        assert!(!not_overflow_1, TEST_ERROR);
        assert!(!not_overflow_2, TEST_ERROR);
    }

    #[test]
    public entry fun test_sqrt() {
        let a = sqrt(1, 100);
        assert!(a == 10, SQRT_ERROR);
        let a = sqrt(1, 1000);
        assert!(a == 31, SQRT_ERROR);
        let a = sqrt(10003, 7);
        assert!(a == 264, SQRT_ERROR);
        let a = sqrt(999999999999999, 1);
        assert!(a == 31622776, SQRT_ERROR);
    }

    #[test]
    public entry fun test_quote() {
        let a = quote(123, 456, 789);
        assert!(a == 212, QUOTE_ERROR);
    }

    #[test]
    public entry fun test_get_amount_out() {
        let a = get_amount_out(123456789, 456789123, 789123456, 30);
        assert!(a == 167502115, TEST_ERROR);
    }

    #[test]
    public entry fun test_get_amount_in() {
        let a = get_amount_in(123456789, 456789123, 789123456, 30);
        assert!(a == 84972572, TEST_ERROR);
    }

    #[test_only]
    struct TestCoinA {}
    #[test_only]
    struct TestCoinB {}
    #[test_only]
    struct TestCoinAA {}

    #[test]
    public entry fun test_compare() {
        let a = compare<TestCoinA, TestCoinB>();
        assert!(a == true, TEST_ERROR);
        let a = compare<TestCoinB, TestCoinAA>();
        assert!(a == true, TEST_ERROR);
    }
}