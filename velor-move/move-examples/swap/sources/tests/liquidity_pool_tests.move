#[test_only]
module swap::liquidity_pool_tests {
    use velor_framework::fungible_asset::{Self, FungibleAsset};
    use velor_framework::object::Object;
    use velor_framework::primary_fungible_store;
    use swap::liquidity_pool::{Self, LiquidityPool};
    use swap::test_helpers;
    use std::signer;
    use std::vector;

    #[test(lp_1 = @0xcafe, lp_2 = @0xdead)]
    fun test_e2e_volatile(lp_1: &signer, lp_2: &signer) {
        test_helpers::set_up(lp_1);
        let is_stable = false;
        let (pool, tokens_2, tokens_1) = create_pool(lp_1, is_stable);

        // Add liquidity to the pool
        add_liquidity(lp_1, &mut tokens_1, &mut tokens_2, 100000, 200000, is_stable);
        verify_reserves(pool, 100000, 200000);
        add_liquidity(lp_1, &mut tokens_1, &mut tokens_2, 100000, 100000, is_stable);
        verify_reserves(pool, 200000, 300000);

        // Swap tokens
        let (amount_out_1, fees_2) = swap_and_verify(lp_1, pool, &mut tokens_2, 10000);
        let (amount_out_2, fees_1) = swap_and_verify(lp_1, pool, &mut tokens_1, 10000);
        let expected_total_reserves_1 = ((200000 - amount_out_1 - fees_1 + 10000) as u128);
        let expected_total_reserves_2 = ((300000 - amount_out_2 - fees_2 + 10000) as u128);
        verify_reserves(pool, (expected_total_reserves_1 as u64), (expected_total_reserves_2 as u64));

        // Remove half of the liquidity. Should receive the proportional amounts of tokens back.
        let lp_tokens_to_withdraw = ((primary_fungible_store::balance(signer::address_of(lp_1), pool) / 2) as u128);
        let total_lp_supply = liquidity_pool::lp_token_supply(pool);
        let expected_liq_1 = expected_total_reserves_1 * lp_tokens_to_withdraw / total_lp_supply;
        let expected_liq_2 = expected_total_reserves_2 * lp_tokens_to_withdraw / total_lp_supply;
        let (liq_1, liq_2) = liquidity_pool::burn(
            lp_1,
            fungible_asset::asset_metadata(&tokens_1),
            fungible_asset::asset_metadata(&tokens_2),
            is_stable,
            (lp_tokens_to_withdraw as u64),
        );
        assert!(fungible_asset::amount(&liq_1) == (expected_liq_1 as u64), 0);
        assert!(fungible_asset::amount(&liq_2) == (expected_liq_2 as u64), 0);
        fungible_asset::merge(&mut tokens_1, liq_1);
        fungible_asset::merge(&mut tokens_2, liq_2);

        // There's a second account here - the lp_2 who tries to create the store for the LP token before hand.
        let lp_2_address = signer::address_of(lp_2);
        primary_fungible_store::ensure_primary_store_exists(lp_2_address, pool);
        assert!(primary_fungible_store::primary_store_exists(lp_2_address, pool), 0);
        assert!(!primary_fungible_store::is_frozen(lp_2_address, pool), 0);

        // Original LP transfers all LP tokens to the lp_2, which should also automatically locks their token store
        // to ensure they have to call liquidity_pool::transfer to do the actual transfer.
        let lp_1_address = signer::address_of(lp_1);
        let lp_balance = primary_fungible_store::balance(lp_1_address, pool);
        liquidity_pool::transfer(lp_1, pool, lp_2_address, lp_balance);
        assert!(primary_fungible_store::is_frozen(lp_2_address, pool), 0);
        assert!(primary_fungible_store::balance(lp_2_address, pool) == lp_balance, 0);

        // The original LP should still receive the fees for the swap that already happens, which is a bit less due to
        // the min liquidity.
        let (claimed_1, claimed_2) = liquidity_pool::claim_fees(lp_1, pool);
        assert!(fungible_asset::amount(&claimed_1) == fees_1 - 1, 0);
        assert!(fungible_asset::amount(&claimed_2) == fees_2 - 1, 0);
        primary_fungible_store::deposit(lp_1_address, claimed_1);
        primary_fungible_store::deposit(lp_1_address, claimed_2);

        // No more rewards to claim.
        let (remaining_1, remaining_2) = liquidity_pool::claimable_fees(lp_1_address, pool);
        assert!(remaining_1 == 0, 0);
        assert!(remaining_2 == 0, 0);

        // More swaps happen. Now the new LP should receive fees. Original LP shouldn't receive anymore.
        let (_, fees_1) = swap_and_verify(lp_1, pool, &mut tokens_1, 10000);
        let (claimed_1, claimed_2) = liquidity_pool::claim_fees(lp_2, pool);
        assert!(fungible_asset::amount(&claimed_1) == fees_1 - 1, 0);
        assert!(fungible_asset::amount(&claimed_2) == 0, 0);
        let (original_claimable_1, original_claimable_2) = liquidity_pool::claimable_fees(lp_1_address, pool);
        assert!(original_claimable_1 == 0 && original_claimable_2 == 0, 0);
        primary_fungible_store::deposit(lp_1_address, claimed_1);
        primary_fungible_store::deposit(lp_1_address, claimed_2);

        primary_fungible_store::deposit(lp_1_address, tokens_1);
        primary_fungible_store::deposit(lp_1_address, tokens_2);
    }

    #[test(lp_1 = @0xcafe)]
    fun test_e2e_stable(lp_1: &signer) {
        test_helpers::set_up(lp_1);
        let is_stable = true;
        let (pool, tokens_2, tokens_1) = create_pool(lp_1, is_stable);
        add_liquidity(lp_1, &mut tokens_1, &mut tokens_2, 100000, 200000, is_stable);
        swap_and_verify(lp_1, pool, &mut tokens_2, 10000);
        swap_and_verify(lp_1, pool, &mut tokens_1, 10000);

        let lp_1_address = signer::address_of(lp_1);
        primary_fungible_store::deposit(lp_1_address, tokens_1);
        primary_fungible_store::deposit(lp_1_address, tokens_2);
    }

    fun verify_reserves(pool: Object<LiquidityPool>, expected_1: u64, expected_2: u64) {
        let (reserves_1, reserves_2) = liquidity_pool::pool_reserves(pool);
        assert!(reserves_1 == expected_1, 0);
        assert!(reserves_2 == expected_2, 0);
    }

    public fun swap_and_verify(
        lp_1: &signer,
        pool: Object<LiquidityPool>,
        tokens: &mut FungibleAsset,
        amount_in: u64,
    ): (u64, u64) {
        let (reserves_1, reserves_2) = liquidity_pool::pool_reserves(pool);
        let pool_assets = liquidity_pool::supported_inner_assets(pool);
        if (fungible_asset::asset_metadata(tokens) != *vector::borrow(&pool_assets, 0)) {
            (reserves_1, reserves_2) = (reserves_2, reserves_1);
        };
        let out = liquidity_pool::swap(pool, fungible_asset::extract(tokens, amount_in));

        let fees_amount = liquidity_pool::swap_fee_bps(pool) * amount_in / 10000;
        amount_in = amount_in - fees_amount;
        let actual_amount = fungible_asset::amount(&out);
        if (!liquidity_pool::is_stable(pool)) {
            assert!(actual_amount == amount_in * reserves_2 / (reserves_1 + amount_in), 0);
        };
        primary_fungible_store::deposit(signer::address_of(lp_1), out);
        (actual_amount, fees_amount)
    }

    public fun add_liquidity(
        lp: &signer,
        tokens_1: &mut FungibleAsset,
        tokens_2: &mut FungibleAsset,
        amount_1: u64,
        amount_2: u64,
        is_stable: bool,
    ) {
        liquidity_pool::mint(
            lp,
            fungible_asset::extract(tokens_1, amount_1),
            fungible_asset::extract(tokens_2, amount_2),
            is_stable,
        );
    }

    public fun create_pool(lp_1: &signer, is_stable: bool): (Object<LiquidityPool>, FungibleAsset, FungibleAsset) {
        let tokens_1 = test_helpers::create_fungible_asset_and_mint(lp_1, b"test1", 10000000);
        let tokens_2 = test_helpers::create_fungible_asset_and_mint(lp_1, b"test2", 10000000);
        let pool = liquidity_pool::create(
            fungible_asset::asset_metadata(&tokens_1),
            fungible_asset::asset_metadata(&tokens_2),
            is_stable,
        );
        (pool, tokens_1, tokens_2)
    }
}
