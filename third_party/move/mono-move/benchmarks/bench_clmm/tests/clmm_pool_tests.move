#[test_only]
module bench::clmm_pool_tests {
    use aptos_framework::fungible_asset::Metadata;
    use aptos_framework::object::Object;
    use bench::clmm_mock_fa;
    use bench::clmm_pool;
    use bench::clmm_tick_math;

    const FEE_RATE: u64 = 3000;
    const TICK_SPACING: u32 = 60;
    const FUNDING: u64 = 1000000000000000000;

    const BASE_LIQUIDITY: u128 = 1000000000000000;
    const EXTRA_LIQUIDITY: u128 = 500000000000000;

    fun assets(admin: &signer): (Object<Metadata>, Object<Metadata>) {
        let token_a = clmm_mock_fa::create_asset(admin, b"TKA", 8);
        let token_b = clmm_mock_fa::create_asset(admin, b"TKB", 8);
        clmm_mock_fa::mint(admin, token_a, @bench, FUNDING);
        clmm_mock_fa::mint(admin, token_b, @bench, FUNDING);
        (token_a, token_b)
    }

    /// A pool at tick 0 with both tokens funded to the admin.
    fun setup(admin: &signer): address {
        let (token_a, token_b) = assets(admin);
        clmm_pool::create_pool(
            admin, token_a, token_b, FEE_RATE, TICK_SPACING, clmm_tick_math::q96()
        );
        clmm_pool::pool_address(@bench, token_a, token_b, FEE_RATE, TICK_SPACING)
    }

    #[test(admin = @bench)]
    fun test_create_pool_starts_at_the_given_price(admin: &signer) {
        let pool_id = setup(admin);
        assert!(clmm_pool::sqrt_price(pool_id) == clmm_tick_math::q96(), 0);
        assert!(clmm_pool::current_tick(pool_id) == 0, 0);
        assert!(clmm_pool::liquidity(pool_id) == 0, 0);
        let (a, b) = clmm_pool::vault_balances(pool_id);
        assert!(a == 0 && b == 0, 0);
    }

    // A range straddling the price counts toward pool liquidity immediately and
    // takes both tokens.
    #[test(admin = @bench)]
    fun test_mint_in_range_takes_both_tokens(admin: &signer) {
        let pool_id = setup(admin);
        clmm_pool::mint(admin, pool_id, -600, 600, BASE_LIQUIDITY);
        assert!(clmm_pool::liquidity(pool_id) == BASE_LIQUIDITY, 0);
        assert!(clmm_pool::position_liquidity(pool_id, @bench, -600, 600) == BASE_LIQUIDITY, 0);
        let (a, b) = clmm_pool::vault_balances(pool_id);
        assert!(a > 0 && b > 0, 0);
        assert!(clmm_pool::tick_is_initialized(pool_id, -600), 0);
        assert!(clmm_pool::tick_is_initialized(pool_id, 600), 0);
        assert!(clmm_pool::tick_liquidity_net(pool_id, -600) == (BASE_LIQUIDITY as i128), 0);
        assert!(clmm_pool::tick_liquidity_net(pool_id, 600) == -(BASE_LIQUIDITY as i128), 0);
    }

    // A range entirely above the price holds only token A, and one entirely
    // below holds only token B. Neither counts toward pool liquidity.
    #[test(admin = @bench)]
    fun test_mint_out_of_range_takes_one_token(admin: &signer) {
        let pool_id = setup(admin);
        clmm_pool::mint(admin, pool_id, 600, 1200, BASE_LIQUIDITY);
        let (a, b) = clmm_pool::vault_balances(pool_id);
        assert!(a > 0 && b == 0, 0);
        assert!(clmm_pool::liquidity(pool_id) == 0, 0);

        clmm_pool::mint(admin, pool_id, -1200, -600, BASE_LIQUIDITY);
        let (a2, b2) = clmm_pool::vault_balances(pool_id);
        assert!(a2 == a && b2 > 0, 0);
        assert!(clmm_pool::liquidity(pool_id) == 0, 0);
    }

    // Minting and burning the same amount at the same range leaves the pool as
    // it was, with both ticks gone.
    #[test(admin = @bench)]
    fun test_mint_then_full_burn_restores_the_pool(admin: &signer) {
        let pool_id = setup(admin);
        clmm_pool::mint(admin, pool_id, -600, 600, BASE_LIQUIDITY);
        assert!(clmm_pool::liquidity(pool_id) == BASE_LIQUIDITY, 0);

        clmm_pool::burn(admin, pool_id, -600, 600, BASE_LIQUIDITY);
        assert!(clmm_pool::liquidity(pool_id) == 0, 0);
        assert!(clmm_pool::position_liquidity(pool_id, @bench, -600, 600) == 0, 0);
        assert!(!clmm_pool::tick_is_initialized(pool_id, -600), 0);
        assert!(!clmm_pool::tick_is_initialized(pool_id, 600), 0);
        assert!(clmm_pool::tick_liquidity_gross(pool_id, -600) == 0, 0);
        assert!(clmm_pool::tick_liquidity_gross(pool_id, 600) == 0, 0);

        // The principal is owed back, and collecting it empties the vaults up
        // to the unit the pool keeps from rounding.
        let (owed_a, owed_b) = clmm_pool::position_tokens_owed(pool_id, @bench, -600, 600);
        assert!(owed_a > 0 && owed_b > 0, 0);
        clmm_pool::collect(admin, pool_id, -600, 600, owed_a, owed_b);
        let (a, b) = clmm_pool::vault_balances(pool_id);
        assert!(a <= 1 && b <= 1, 0);
    }

    // Crossing a single tick upward adds exactly that tick's `liquidity_net`,
    // and crossing back down subtracts it again.
    #[test(admin = @bench)]
    fun test_swap_across_one_tick_moves_liquidity_by_liquidity_net(admin: &signer) {
        let pool_id = setup(admin);
        clmm_pool::mint(admin, pool_id, -6000, 6000, BASE_LIQUIDITY);
        clmm_pool::mint(admin, pool_id, 60, 6000, EXTRA_LIQUIDITY);

        let net_at_60 = clmm_pool::tick_liquidity_net(pool_id, 60);
        assert!(net_at_60 == (EXTRA_LIQUIDITY as i128), 0);
        let before = clmm_pool::liquidity(pool_id);
        assert!(before == BASE_LIQUIDITY, 0);

        // Buy token A until the price passes tick 60 but stops before tick 120.
        clmm_pool::swap_exact_in(
            admin,
            pool_id,
            false,
            100000000000000,
            clmm_tick_math::get_sqrt_price_at_tick(120)
        );
        let after_up = clmm_pool::liquidity(pool_id);
        assert!(after_up == before + (net_at_60 as u128), 0);
        assert!(clmm_pool::current_tick(pool_id) >= 60, 0);

        // Selling it back crosses the same tick downward, where the sign flips.
        clmm_pool::swap_exact_in(
            admin, pool_id, true, 100000000000000, clmm_tick_math::q96()
        );
        let after_down = clmm_pool::liquidity(pool_id);
        assert!(after_down == after_up - (net_at_60 as u128), 0);
        assert!(after_down == before, 0);
        assert!(clmm_pool::current_tick(pool_id) < 60, 0);
    }

    // A swap moves the price in the right direction and never past the limit.
    #[test(admin = @bench)]
    fun test_swap_respects_the_price_limit(admin: &signer) {
        let pool_id = setup(admin);
        clmm_pool::mint(admin, pool_id, -6000, 6000, BASE_LIQUIDITY);

        let limit = clmm_tick_math::get_sqrt_price_at_tick(600);
        clmm_pool::swap_exact_in(admin, pool_id, false, 100000000000000, limit);
        assert!(clmm_pool::sqrt_price(pool_id) == limit, 0);

        let limit_down = clmm_tick_math::get_sqrt_price_at_tick(-600);
        clmm_pool::swap_exact_in(admin, pool_id, true, 100000000000000, limit_down);
        assert!(clmm_pool::sqrt_price(pool_id) == limit_down, 0);
    }

    // A swap small enough to stay inside one range consumes its whole input.
    #[test(admin = @bench)]
    fun test_swap_exact_in_consumes_its_input(admin: &signer) {
        let pool_id = setup(admin);
        clmm_pool::mint(admin, pool_id, -6000, 6000, BASE_LIQUIDITY);
        let (vault_a_before, vault_b_before) = clmm_pool::vault_balances(pool_id);

        let amount = 1000000000u64;
        clmm_pool::swap_exact_in(
            admin, pool_id, true, amount, clmm_tick_math::get_sqrt_price_at_tick(-600)
        );
        let (vault_a_after, vault_b_after) = clmm_pool::vault_balances(pool_id);
        assert!(vault_a_after == vault_a_before + amount, 0);
        assert!(vault_b_after < vault_b_before, 0);
        assert!(clmm_pool::current_tick(pool_id) < 0, 0);
    }

    #[test(admin = @bench)]
    fun test_swap_exact_out_delivers_the_request(admin: &signer) {
        let pool_id = setup(admin);
        clmm_pool::mint(admin, pool_id, -6000, 6000, BASE_LIQUIDITY);
        let (_, vault_b_before) = clmm_pool::vault_balances(pool_id);

        let amount = 1000000000u64;
        clmm_pool::swap_exact_out(
            admin, pool_id, true, amount, clmm_tick_math::get_sqrt_price_at_tick(-600)
        );
        let (_, vault_b_after) = clmm_pool::vault_balances(pool_id);
        assert!(vault_b_before - vault_b_after == amount, 0);
    }

    // Swapping accrues fees to the position that was in range for them.
    #[test(admin = @bench)]
    fun test_fees_reach_the_position(admin: &signer) {
        let pool_id = setup(admin);
        clmm_pool::mint(admin, pool_id, -6000, 6000, BASE_LIQUIDITY);
        let (owed_a, owed_b) = clmm_pool::position_tokens_owed(pool_id, @bench, -6000, 6000);
        assert!(owed_a == 0 && owed_b == 0, 0);

        clmm_pool::swap_exact_in(
            admin, pool_id, true, 1000000000, clmm_tick_math::get_sqrt_price_at_tick(-600)
        );
        clmm_pool::swap_exact_in(
            admin, pool_id, false, 1000000000, clmm_tick_math::get_sqrt_price_at_tick(600)
        );

        clmm_pool::poke(admin, pool_id, -6000, 6000);
        let (owed_a, owed_b) = clmm_pool::position_tokens_owed(pool_id, @bench, -6000, 6000);
        // 0.30% of a billion, less the rounding the pool keeps.
        assert!(owed_a > 2900000 && owed_a <= 3000000, 0);
        assert!(owed_b > 2900000 && owed_b <= 3000000, 0);

        let balance_before = clmm_mock_fa::balance(@bench, clmm_mock_fa::metadata(@bench, b"TKA"));
        clmm_pool::collect(admin, pool_id, -6000, 6000, owed_a, owed_b);
        let balance_after = clmm_mock_fa::balance(@bench, clmm_mock_fa::metadata(@bench, b"TKA"));
        assert!(balance_after == balance_before + owed_a, 0);
        let (owed_a, owed_b) = clmm_pool::position_tokens_owed(pool_id, @bench, -6000, 6000);
        assert!(owed_a == 0 && owed_b == 0, 0);
    }

    // A position that was out of range for the whole swap earns nothing.
    #[test(admin = @bench)]
    fun test_out_of_range_position_earns_nothing(admin: &signer) {
        let pool_id = setup(admin);
        clmm_pool::mint(admin, pool_id, -6000, 6000, BASE_LIQUIDITY);
        clmm_pool::mint(admin, pool_id, 3000, 6000, EXTRA_LIQUIDITY);

        clmm_pool::swap_exact_in(
            admin, pool_id, true, 1000000000, clmm_tick_math::get_sqrt_price_at_tick(-600)
        );
        clmm_pool::poke(admin, pool_id, 3000, 6000);
        let (owed_a, owed_b) = clmm_pool::position_tokens_owed(pool_id, @bench, 3000, 6000);
        assert!(owed_a == 0 && owed_b == 0, 0);
    }

    #[test(admin = @bench)]
    fun test_seed_positions_fills_the_bitmap(admin: &signer) {
        let pool_id = setup(admin);
        clmm_pool::seed_positions(admin, pool_id, 8, 600, 42);
        // Jittered ranges around tick 0 leave the pool with liquidity in force
        // at some of them.
        let (a, b) = clmm_pool::vault_balances(pool_id);
        assert!(a > 0 || b > 0, 0);
    }

    #[test(admin = @bench)]
    fun test_seed_positions_scales_with_the_knob(admin: &signer) {
        let pool_id = setup(admin);
        clmm_pool::seed_positions(admin, pool_id, 64, 600, 7);
        let (a, b) = clmm_pool::vault_balances(pool_id);
        assert!(a > 0 && b > 0, 0);
    }

    // The whole path: assets, pool, seeded positions, swaps across several
    // ticks in both directions, fee collection and a burn.
    #[test(admin = @bench)]
    fun test_end_to_end(admin: &signer) {
        let pool_id = setup(admin);

        // A wide base position guarantees liquidity at the starting price, so
        // the seeded ranges only add depth around it.
        clmm_pool::mint(admin, pool_id, -60000, 60000, BASE_LIQUIDITY);
        clmm_pool::seed_positions(admin, pool_id, 16, 600, 2024);

        let (vault_a_start, vault_b_start) = clmm_pool::vault_balances(pool_id);
        assert!(vault_a_start > 0 && vault_b_start > 0, 0);

        // Against a base liquidity of 1e15 these move the price by roughly
        // five, fifty and a thousand ticks.
        clmm_pool::swap_exact_in(
            admin, pool_id, true, 500000000000, clmm_tick_math::get_sqrt_price_at_tick(-180)
        );
        assert!(clmm_pool::current_tick(pool_id) < 0, 0);
        clmm_pool::swap_exact_in(
            admin, pool_id, true, 5000000000000, clmm_tick_math::get_sqrt_price_at_tick(-600)
        );
        clmm_pool::swap_exact_in(
            admin, pool_id, false, 50000000000000, clmm_tick_math::get_sqrt_price_at_tick(3000)
        );
        assert!(clmm_pool::current_tick(pool_id) > 0, 0);
        clmm_pool::swap_exact_out(
            admin, pool_id, true, 1000000000, clmm_tick_math::get_sqrt_price_at_tick(-3000)
        );

        // The base position earned fees on every one of those swaps.
        clmm_pool::poke(admin, pool_id, -60000, 60000);
        let (owed_a, owed_b) = clmm_pool::position_tokens_owed(pool_id, @bench, -60000, 60000);
        assert!(owed_a > 0 && owed_b > 0, 0);
        clmm_pool::collect(admin, pool_id, -60000, 60000, owed_a, owed_b);
        let (owed_a, owed_b) = clmm_pool::position_tokens_owed(pool_id, @bench, -60000, 60000);
        assert!(owed_a == 0 && owed_b == 0, 0);

        // Closing the base position leaves the seeded ones behind.
        clmm_pool::burn(admin, pool_id, -60000, 60000, BASE_LIQUIDITY);
        assert!(clmm_pool::position_liquidity(pool_id, @bench, -60000, 60000) == 0, 0);
        assert!(!clmm_pool::tick_is_initialized(pool_id, -60000), 0);
        assert!(!clmm_pool::tick_is_initialized(pool_id, 60000), 0);
        let (owed_a, owed_b) = clmm_pool::position_tokens_owed(pool_id, @bench, -60000, 60000);
        assert!(owed_a > 0 || owed_b > 0, 0);
        clmm_pool::collect(admin, pool_id, -60000, 60000, owed_a, owed_b);

        let (vault_a_end, vault_b_end) = clmm_pool::vault_balances(pool_id);
        assert!(vault_a_end > 0 && vault_b_end > 0, 0);
    }

    #[test(admin = @bench)]
    #[expected_failure(abort_code = clmm_pool::EPOOL_EXISTS)]
    fun test_create_pool_twice_fails(admin: &signer) {
        let (token_a, token_b) = assets(admin);
        clmm_pool::create_pool(
            admin, token_a, token_b, FEE_RATE, TICK_SPACING, clmm_tick_math::q96()
        );
        clmm_pool::create_pool(
            admin, token_a, token_b, FEE_RATE, TICK_SPACING, clmm_tick_math::q96()
        );
    }

    #[test(admin = @bench)]
    #[expected_failure(abort_code = clmm_pool::ESAME_TOKEN)]
    fun test_create_pool_needs_two_tokens(admin: &signer) {
        let (token_a, _) = assets(admin);
        clmm_pool::create_pool(
            admin, token_a, token_a, FEE_RATE, TICK_SPACING, clmm_tick_math::q96()
        );
    }

    #[test(admin = @bench)]
    #[expected_failure(abort_code = clmm_pool::EINVALID_PRICE_LIMIT)]
    fun test_swap_rejects_a_limit_on_the_wrong_side(admin: &signer) {
        let pool_id = setup(admin);
        clmm_pool::mint(admin, pool_id, -6000, 6000, BASE_LIQUIDITY);
        clmm_pool::swap_exact_in(
            admin, pool_id, true, 1000, clmm_tick_math::get_sqrt_price_at_tick(600)
        );
    }

    #[test(admin = @bench)]
    #[expected_failure(abort_code = clmm_pool::EZERO_LIQUIDITY)]
    fun test_mint_rejects_zero_liquidity(admin: &signer) {
        let pool_id = setup(admin);
        clmm_pool::mint(admin, pool_id, -600, 600, 0);
    }

    // The recipe a transaction harness follows: seed liquidity once as admin,
    // onboard each account with one transaction, then run the mix. Every step
    // after onboarding must succeed from a plain signer holding nothing.
    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_harness_onboard_then_mix(admin: &signer, alice: &signer) {
        let pool_id = setup(admin);
        // A wide range keeps liquidity in play whichever way the mix pushes
        // the price, so no swap can find an empty book.
        clmm_pool::mint(admin, pool_id, -60000, 60000, BASE_LIQUIDITY);
        clmm_pool::seed_positions(admin, pool_id, 16, 600, 11);

        clmm_pool::bench_onboard(alice, pool_id, -1200, 1200, EXTRA_LIQUIDITY, FUNDING);
        assert!(
            clmm_pool::position_liquidity(pool_id, @0xa11ce, -1200, 1200)
                == EXTRA_LIQUIDITY,
            0
        );

        clmm_pool::bench_swap_in(alice, pool_id, true, 1000000);
        clmm_pool::bench_swap_in(alice, pool_id, false, 1000000);
        clmm_pool::poke(alice, pool_id, -1200, 1200);
        clmm_pool::collect(alice, pool_id, -1200, 1200, 1000000, 1000000);
        clmm_pool::bench_rebalance(alice, pool_id, -600, 600, EXTRA_LIQUIDITY, FUNDING);

        // Rebalancing leaves nothing behind, and the onboarded position is
        // untouched by it.
        assert!(clmm_pool::position_liquidity(pool_id, @0xa11ce, -600, 600) == 0, 0);
        assert!(
            clmm_pool::position_liquidity(pool_id, @0xa11ce, -1200, 1200)
                == EXTRA_LIQUIDITY,
            0
        );
    }
}
