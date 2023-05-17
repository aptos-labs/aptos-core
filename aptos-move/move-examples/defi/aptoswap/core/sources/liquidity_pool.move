/// This module demonstrates how to create a core module of AMM following uniswap v2 algorithm with Aptos fungible
/// asset framework.
module pool_manager::liquidity_pool {
    use aptos_framework::fungible_asset::{Metadata, FungibleStore, FungibleAsset, deposit};
    use aptos_framework::object::{Self, ExtendRef, Object, generate_signer_for_extending, generate_extend_ref};
    use aptos_framework::fungible_asset;
    use aptos_framework::primary_fungible_store;
    use aptos_std::comparator;
    use aptos_std::math128;
    use aptos_std::math64;
    use aptos_std::smart_table::{Self, SmartTable};
    use aptos_std::string_utils;
    use std::bcs;
    use std::error;
    use std::option::{Self, Option};
    use std::string::{Self, String};
    use std::vector;

    const MINIMAL_LIQUIDITY: u64 = 1000;

    // fee is basis point based so the scale is 10000.
    const FEE_SCALE: u64 = 10000;

    /// A meaningful swap pair must be different assets.
    const ERR_SWAP_PAIR_CANNOT_BE_THE_SAME: u64 = 1;
    /// The liquidity pool already exists.
    const ERR_POOL_EXISTENCE: u64 = 2;
    /// The minted liquidity is too small.
    const ERR_NOT_ENOUGH_LIQUIDITY: u64 = 3;
    /// The number of lp tokens to burn is too small to redeem both assets.
    const ERR_NOT_ENOUGH_LP_TOKEN_TO_BURN: u64 = 4;
    /// The number of assets to swap cannot be zero.
    const ERR_ZERO_COIN_TO_SWAP: u64 = 5;
    /// The swap function arguments are not valid because it will result in lower liquidity after the swap.
    const ERR_INCORRECT_SWAP_DUE_TO_REDUCED_LIQUIDITY: u64 = 6;

    struct LiquidityPool has key {
        extend_ref: ExtendRef,
        /// a vector of length 2, i.e. an object pair, whose addresses must be lexicographically ordered.
        store_pair: vector<Object<FungibleStore>>,
        lp_token_mint_ref: fungible_asset::MintRef,
        lp_token_burn_ref: fungible_asset::BurnRef,
        // basis points
        fee: u64,
    }

    /// The onchain index of the liquidity pools of different pairs
    struct LiquidityPoolManager has key {
        extend_ref: ExtendRef,
        map: SmartTable<vector<Object<Metadata>>, Object<LiquidityPool>>
    }

    /// Intialize the module in pool manager object
    fun init_module(admin: &signer) {
        let extend_ref = aptoswap::init::retrieve_extend_ref(admin);
        let pool_manager_obj_signer = object::generate_signer_for_extending(&extend_ref);
        move_to(&pool_manager_obj_signer, LiquidityPoolManager {
            extend_ref,
            map: smart_table::new()
        })
    }

    /// Create liquidity pool `X`/`Y`.
    public fun create(acc: &signer, x: Object<Metadata>, y: Object<Metadata>) acquires LiquidityPoolManager {
        if (!is_valid_pair(x, y)) {
            return create(acc, y, x)
        };
        assert!(option::is_none(&get_liquidity_pool(x, y)), error::already_exists(ERR_POOL_EXISTENCE));

        let (lp_token_name, lp_token_symbol) = lp_token_name_and_symbol(x, y);

        let pool_manager_signer = &get_pool_account_signer();
        let pool_cref = &object::create_named_object(pool_manager_signer, pool_address_seed(x, y));
        let pool_signer = &object::generate_signer(pool_cref);

        // Create lp fungible token metadata object reusing the same liquidity pool object.
        let (lp_token_mint_ref, lp_token_burn_ref) = {
            primary_fungible_store::create_primary_store_enabled_fungible_asset(
                pool_cref,
                option::none(),
                lp_token_name,
                lp_token_symbol,
                6,
                string::utf8(b"http://aptoswap.com/favicon.ico"),
                string::utf8(b"http://aptoswap.com/")
            );
            (
                fungible_asset::generate_mint_ref(pool_cref),
                fungible_asset::generate_burn_ref(pool_cref),
            )
        };

        let x_store = fungible_asset::create_store(&object::create_object_from_object(pool_signer), x);
        let y_store = fungible_asset::create_store(&object::create_object_from_object(pool_signer), y);

        // create store reusing the same liquidity pool object to hold min_liquidity
        fungible_asset::create_store(pool_cref, fungible_asset::mint_ref_metadata(&lp_token_mint_ref));
        let liquidity_pool = LiquidityPool {
            extend_ref: generate_extend_ref(pool_cref),
            store_pair: vector[x_store, y_store],
            lp_token_mint_ref,
            lp_token_burn_ref,
            fee: 0
        };
        move_to(pool_signer, liquidity_pool);
        let map = &mut borrow_global_mut<LiquidityPoolManager>(@pool_manager).map;
        smart_table::add(map, vector[x, y], object::object_from_constructor_ref<LiquidityPool>(pool_cref));
    }

    /// Mint new liquidity coins.
    /// * `coin_x` - coin X to add to liquidity reserves.
    /// * `coin_y` - coin Y to add to liquidity reserves.
    /// Returns LP coins: `Coin<LP<X, Y, Curve>>`.
    public fun mint(
        coin_x: FungibleAsset,
        coin_y: FungibleAsset
    ): FungibleAsset acquires LiquidityPoolManager, LiquidityPool {
        let x = fungible_asset::metadata_from_asset(&coin_x);
        let y = fungible_asset::metadata_from_asset(&coin_y);
        if (!is_valid_pair(x, y)) {
            return mint(coin_y, coin_x)
        };

        let pool = borrow_lp(x, y);
        let x_store = *vector::borrow(&pool.store_pair, 0);
        let y_store = *vector::borrow(&pool.store_pair, 1);
        let x_reserve_size = fungible_asset::balance(x_store);
        let y_reserve_size = fungible_asset::balance(y_store);

        let x_value = fungible_asset::amount(&coin_x);
        let y_value = fungible_asset::amount(&coin_y);

        let lp_token_supply = lp_token_supply(pool);
        let provided_liq = if (lp_token_supply == 0) {
            let initial_liq = (math128::sqrt((x_value as u128) * (y_value as u128)) as u64);
            let min_liquidity = fungible_asset::mint(&pool.lp_token_mint_ref, MINIMAL_LIQUIDITY);
            primary_fungible_store::deposit(object::address_from_extend_ref(&pool.extend_ref), min_liquidity);
            initial_liq - MINIMAL_LIQUIDITY
        } else {
            let x_liq = math64::mul_div(x_value, (lp_token_supply as u64), x_reserve_size);
            let y_liq = math64::mul_div(y_value, (lp_token_supply as u64), y_reserve_size);
            math64::min(x_liq, y_liq)
        };
        assert!(provided_liq > 0, ERR_NOT_ENOUGH_LIQUIDITY);

        fungible_asset::deposit(x_store, coin_x);
        fungible_asset::deposit(y_store, coin_y);

        // mint new lp tokens(fa)
        fungible_asset::mint(&pool.lp_token_mint_ref, provided_liq)
    }

    /// Burn liquidity coins (LP) and get back X and Y coins from reserves.
    /// * `lp tokens` - LP tokens to burn.
    /// Returns both X and Y coins - `(Coin<X>, Coin<Y>)`.
    public fun burn(
        lp_tokens: FungibleAsset
    ): (FungibleAsset, FungibleAsset) acquires LiquidityPool {
        let lp_token_metadata = fungible_asset::metadata_from_asset(&lp_tokens);
        let lp = object::convert<Metadata, LiquidityPool>(lp_token_metadata);
        let pool = borrow_global<LiquidityPool>(object::object_address(&lp));

        let amount_to_burn = fungible_asset::amount(&lp_tokens);
        let lp_token_supply = lp_token_supply(pool);
        let x_store = *vector::borrow(&pool.store_pair, 0);
        let y_store = *vector::borrow(&pool.store_pair, 1);
        let x_reserve_size = fungible_asset::balance(x_store);
        let y_reserve_size = fungible_asset::balance(y_store);

        // Compute x, y coin values for provided lp_coins value
        let x_amount_to_redeem = (math128::mul_div(
            (amount_to_burn as u128),
            (x_reserve_size as u128),
            lp_token_supply
        ) as u64);
        let y_amount_to_redeem = (math128::mul_div(
            (amount_to_burn as u128),
            (y_reserve_size as u128),
            lp_token_supply
        ) as u64);
        assert!(x_amount_to_redeem != 0 && y_amount_to_redeem != 0, ERR_NOT_ENOUGH_LP_TOKEN_TO_BURN);

        let pool_signer = generate_signer_for_extending(&pool.extend_ref);
        // Withdraw those values from reserves
        let x_coin = fungible_asset::withdraw(&pool_signer, x_store, x_amount_to_redeem);
        let y_coin = fungible_asset::withdraw(&pool_signer, y_store, y_amount_to_redeem);

        fungible_asset::burn(&pool.lp_token_burn_ref, lp_tokens);

        (x_coin, y_coin)
    }

    /// Swap a for b.
    /// * `in` - fungible asset a to swap.
    /// * `to` - the type of b.
    /// * `to_amount` - expected amount of the coins of b to get out.
    /// return `to_amount` of b as fungible asset.
    public fun swap(
        in: FungibleAsset,
        to: Object<Metadata>,
        to_amount: u64,
    ): FungibleAsset acquires LiquidityPool {
        let from = fungible_asset::metadata_from_asset(&in);
        let (from_index, pool) = if (is_valid_pair(from, to)) {
            (0, borrow_lp(from, to))
        } else {
            (1, borrow_lp(to, from))
        };
        let from_reserve = fungible_asset::balance(*vector::borrow(&pool.store_pair, from_index));
        let to_reserve = fungible_asset::balance(*vector::borrow(&pool.store_pair, 1 - from_index));
        let k_before_swap = (from_reserve as u256) * (to_reserve as u256) * (FEE_SCALE * FEE_SCALE as u256);
        let (out, k_after_swap_and_fee) = swap_and_calculate_new_k(pool, from_index, in, to_amount);
        assert!(
            k_after_swap_and_fee >= k_before_swap,
            error::invalid_argument(ERR_INCORRECT_SWAP_DUE_TO_REDUCED_LIQUIDITY)
        );
        out
    }

    #[view]
    public fun lp_token_type(
        x: Object<Metadata>,
        y: Object<Metadata>
    ): Object<Metadata> acquires LiquidityPoolManager, LiquidityPool {
        if (!is_valid_pair(x, y)) {
            return lp_token_type(y, x)
        };
        let pool = borrow_lp(x, y);
        fungible_asset::mint_ref_metadata(&pool.lp_token_mint_ref)
    }

    #[view]
    /// Get the deterministic liquidity pool address given two fungible assets.
    public fun liquidity_pool_address(x: Object<Metadata>, y: Object<Metadata>): address {
        if (!is_valid_pair(x, y)) {
            return liquidity_pool_address(y, x)
        };
        object::create_object_address(&@pool_manager, pool_address_seed(x, y))
    }

    #[view]
    /// (x, y) must be sorted before passed in.
    public fun get_liquidity_pool(x: Object<Metadata>, y: Object<Metadata>): Option<Object<LiquidityPool>> {
        if (!exists<LiquidityPoolManager>(@pool_manager)) {
            return option::none()
        };
        let lp_address = liquidity_pool_address(x, y);
        if (exists<LiquidityPool>(lp_address)) {
            option::some(object::address_to_object<LiquidityPool>(lp_address))
        } else {
            option::none()
        }
    }

    /// Generate the seed for deterministic pool address
    /// x and y must be sorted.
    inline fun pool_address_seed(x: Object<Metadata>, y: Object<Metadata>): vector<u8> {
        let s = string_utils::format2(
            &b"aptoswap::liquidity_pool-{}/{}",
            bcs::to_bytes(&object::object_address(&x)),
            bcs::to_bytes(&object::object_address(&y)),
        );
        *string::bytes(&s)
    }

    /// Do the swap minus fee and calculate the new constant k.
    /// Return the swapped coin with the new k (scaled by FEE_SCALE^2).
    inline fun swap_and_calculate_new_k(
        pool: &LiquidityPool,
        i: u64,
        in: FungibleAsset,
        out: u64
    ): (FungibleAsset, u256) {
        let in_store = *vector::borrow(&pool.store_pair, i);
        let out_store = *vector::borrow(&pool.store_pair, 1 - i);
        let in_amount = (fungible_asset::amount(&in) as u128);
        assert!(in_amount > 0, error::invalid_argument(ERR_ZERO_COIN_TO_SWAP));
        deposit(in_store, in);
        let in_store_size_after = (fungible_asset::balance(
            in_store
        ) as u128) * (FEE_SCALE as u128) - in_amount * (pool.fee as u128);
        let pool = generate_signer_for_extending(&pool.extend_ref);
        // Withdraw expected amount from reserves.
        let swapped = fungible_asset::withdraw(&pool, out_store, out);
        let out_store_size_after = (fungible_asset::balance(out_store) as u128) * (FEE_SCALE as u128);
        (swapped, (in_store_size_after as u256) * (out_store_size_after as u256))
    }

    /// Helper inline functions
    inline fun lp_token_supply(pool: &LiquidityPool): u128 {
        let lp_token_metadata_obj = fungible_asset::mint_ref_metadata(&pool.lp_token_mint_ref);
        option::destroy_some(fungible_asset::supply(lp_token_metadata_obj))
    }

    inline fun borrow_lp(
        x: Object<Metadata>,
        y: Object<Metadata>
    ): &LiquidityPool acquires LiquidityPool {
        let lp_opt = get_liquidity_pool(x, y);
        assert!(option::is_some(&lp_opt), error::not_found(ERR_POOL_EXISTENCE));
        let lp_object = option::destroy_some(lp_opt);
        borrow_global<LiquidityPool>(object::object_address(&lp_object))
    }

    /// x must be strictly less than y to be valid.
    inline fun is_valid_pair(x: Object<Metadata>, y: Object<Metadata>): bool {
        let x_addr = object::object_address(&x);
        let y_addr = object::object_address(&y);
        assert!(x_addr != y_addr, error::invalid_argument(ERR_SWAP_PAIR_CANNOT_BE_THE_SAME));
        comparator::is_smaller_than(&comparator::compare(&x_addr, &y_addr))
    }

    inline fun get_pool_account_signer(): signer acquires LiquidityPoolManager {
        let ref = &borrow_global<LiquidityPoolManager>(@pool_manager).extend_ref;
        object::generate_signer_for_extending(ref)
    }

    inline fun lp_token_name_and_symbol(x: Object<Metadata>, y: Object<Metadata>): (String, String) {
        let name = fungible_asset::name(x);
        string::append_utf8(&mut name, b"<>");
        string::append(&mut name, fungible_asset::name(y));

        let symbol = fungible_asset::symbol(x);
        string::append_utf8(&mut symbol, b"/");
        string::append(&mut symbol, fungible_asset::symbol(y));
        (name, symbol)
    }


    #[test_only]
    use example_addr::managed_fungible_asset;
    #[test_only]
    use std::signer;
    #[test_only]
    use aptos_framework::fungible_asset::test_burn;
    #[test_only]
    use aptos_framework::account::create_account_for_test;

    #[test_only]
    fun init_test_module(pool_manager: &signer) {
        let extend_ref = object::generate_extend_ref(&object::create_constructor_ref(pool_manager, false));
        move_to(pool_manager, LiquidityPoolManager {
            extend_ref,
            map: smart_table::new()
        })
    }

    #[test_only]
    fun verify_maybe_reversed_pair(x: FungibleAsset, y: FungibleAsset, x_amount: u64, y_amount: u64) {
        if (is_valid_pair(fungible_asset::asset_metadata(&x), fungible_asset::asset_metadata(&y))) {
            assert!(fungible_asset::amount(&x) == x_amount, 0);
            assert!(fungible_asset::amount(&y) == y_amount, 0);
        } else {
            assert!(fungible_asset::amount(&x) == y_amount, 0);
            assert!(fungible_asset::amount(&y) == x_amount, 0);
        };
        test_burn(x);
        test_burn(y);
    }

    #[test_only]
    fun create_coin(creator: &signer, name: String): Object<Metadata> {
        let constructor_ref = &object::create_named_object(creator, *string::bytes(&name));
        managed_fungible_asset::initialize(
            constructor_ref,
            0,
            name,
            name,
            0, /* decimals */
            string::utf8(b"http://example.com/favicon.ico"), /* icon */
            string::utf8(b"http://example.com"), /* project */
            vector[true, true, true]
        );
        let metadata = object::object_from_constructor_ref<Metadata>(constructor_ref);
        managed_fungible_asset::mint_to_primary_stores(
            creator,
            metadata,
            vector[signer::address_of(creator)],
            vector[100000000]
        );
        metadata
    }

    #[test(pool_manager = @pool_manager, creator = @0xface)]
    fun test_e2e(creator: &signer, pool_manager: &signer) acquires LiquidityPoolManager, LiquidityPool {
        init_test_module(pool_manager);
        let creator_addr = signer::address_of(creator);
        create_account_for_test(creator_addr);
        let x = create_coin(creator, string::utf8(b"X"));
        let y = create_coin(creator, string::utf8(b"Y"));
        let z = create_coin(creator, string::utf8(b"Z"));
        create(creator, x, y);
        create(creator, x, z);
        create(creator, y, z);
        // should takes 10000/10000 coin and gives 9000 lp tokens sqrt(10000*10000)-1000
        {
            let coin_x = primary_fungible_store::withdraw(creator, x, 10000);
            let coin_y = primary_fungible_store::withdraw(creator, y, 10000);
            let lp_tokens = mint(coin_x, coin_y);
            assert!(fungible_asset::amount(&lp_tokens) == 9000, 1);
            primary_fungible_store::deposit(creator_addr, lp_tokens);
        };
        // should takes 1000/100 coin and gives 100 lp tokens 100/10000 * (9000 + 1000)
        {
            let coin_x = primary_fungible_store::withdraw(creator, x, 1000);
            let coin_y = primary_fungible_store::withdraw(creator, y, 100);
            let lp_tokens = mint(coin_x, coin_y);
            assert!(fungible_asset::amount(&lp_tokens) == 100, 1);
            primary_fungible_store::deposit(creator_addr, lp_tokens);
        };
        {
            let lp_token_type = lp_token_type(x, y);
            let lp = primary_fungible_store::withdraw(creator, lp_token_type, 9000);
            let (amount_a, amount_b) = burn(lp);
            verify_maybe_reversed_pair(amount_a, amount_b, 9000, 9801);
        };
    }

    #[test(pool_manager = @pool_manager, creator = @0xface)]
    fun test_swap(creator: &signer, pool_manager: &signer) acquires LiquidityPoolManager, LiquidityPool {
        init_test_module(pool_manager);
        let creator_addr = signer::address_of(creator);
        create_account_for_test(creator_addr);
        let x = create_coin(creator, string::utf8(b"X"));
        let y = create_coin(creator, string::utf8(b"Y"));
        create(creator, x, y);
        // should takes 10000/10000 coin and gives 9000 lp tokens sqrt(10000*10000)-1000
        {
            let coin_x = primary_fungible_store::withdraw(creator, x, 10000);
            let coin_y = primary_fungible_store::withdraw(creator, y, 10000);
            let lp_tokens = mint(coin_x, coin_y);
            primary_fungible_store::deposit(creator_addr, lp_tokens);
        };
        // should takes 1000/100 coin and gives 100 lp tokens 100/10000 * (9000 + 1000)
        {
            let coin_x = primary_fungible_store::withdraw(creator, x, 1000);
            let swapped_coin_y = swap(coin_x, y, 909);
            assert!(fungible_asset::amount(&swapped_coin_y) == 909, 0);
            test_burn(swapped_coin_y);
        };
    }
}
