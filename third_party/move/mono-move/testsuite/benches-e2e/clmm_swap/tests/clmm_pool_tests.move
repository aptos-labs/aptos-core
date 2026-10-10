#[test_only]
module bench::clmm_pool_tests {
    use std::vector;
    use aptos_framework::fungible_asset::Metadata;
    use aptos_framework::object::Object;
    use bench::clmm_assets;
    use bench::clmm_pool;
    use bench::clmm_tick_math;

    const FEE_RATE: u64 = 3000;
    const TICK_SPACING: u32 = 60;
    const FUNDING: u64 = 1000000000000000000;

    const BASE_LIQUIDITY: u128 = 1000000000000000;
    const EXTRA_LIQUIDITY: u128 = 500000000000000;

    /// Seeded boundaries 600 ticks apart at the pool's 60-tick spacing.
    const MID_DENSITY: u32 = 2;
    /// One boundary per 1200 ticks, and one per tick spacing. The high setting
    /// is also what the harness ships, which `test_tick_density_changes_crossings`
    /// checks against the package's own constant.
    const LOW_DENSITY: u32 = 1;
    const HIGH_DENSITY: u32 = 20;

    /// Enough ranges either side that the dense lattice does not run out
    /// before the harness's largest trade does.
    const DENSITY_SEED_POSITIONS: u64 = 44;

    /// Small against every reserve in the chain, so no hop clamps.
    const ROUTE_AMOUNT: u64 = 1000000000;
    /// Far past any reserve, so the first hop must clamp.
    const OVERSIZED_AMOUNT: u64 = 1000000000000000000;

    /// Trades of this size in one direction reach the price bound in a handful
    /// of steps, so a short loop gets to the edge the mix could wander to.
    const DRAINING_TRADE: u64 = 500000000000000000;

    /// Long enough that an unsteered walk of this length would already have
    /// left the seeded lattice, short enough to fit the unit-test gas bound.
    const WALK_SWAPS: u64 = 24;
    /// Crossings per swap a walk at the highest density has to keep averaging.
    /// Measured at about sixteen, so this has room for the walk's own spread.
    const WALK_MIN_CROSSINGS: u64 = 10;

    const LCG_MUL: u64 = 1103515245;
    const LCG_INC: u64 = 12345;
    const LCG_MOD: u64 = 2147483648;

    fun assets(admin: &signer): (Object<Metadata>, Object<Metadata>) {
        let token_a = clmm_assets::create_asset(admin, b"TKA", 8);
        let token_b = clmm_assets::create_asset(admin, b"TKB", 8);
        clmm_assets::mint(admin, token_a, @bench, FUNDING);
        clmm_assets::mint(admin, token_b, @bench, FUNDING);
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

    fun funded_asset(admin: &signer, symbol: vector<u8>): Object<Metadata> {
        let token = clmm_assets::create_asset(admin, symbol, 8);
        clmm_assets::mint(admin, token, @bench, FUNDING);
        token
    }

    /// A pool at tick 0 over `token_a`/`token_b`, backstopped so a swap always
    /// finds liquidity, then seeded at `density`.
    fun seeded_pool(
        admin: &signer,
        token_a: Object<Metadata>,
        token_b: Object<Metadata>,
        n_positions: u64,
        density: u32
    ): address {
        clmm_pool::create_pool(
            admin, token_a, token_b, FEE_RATE, TICK_SPACING, clmm_tick_math::q96()
        );
        let pool_id = clmm_pool::pool_address(@bench, token_a, token_b, FEE_RATE, TICK_SPACING);
        let backstop = clmm_pool::bench_backstop_tick();
        clmm_pool::mint(admin, pool_id, -backstop, backstop, BASE_LIQUIDITY);
        clmm_pool::seed_positions(admin, pool_id, n_positions, density, 11);
        pool_id
    }

    /// Four assets chained through three pools, which is the shape a routed
    /// swap needs: every hop's output token is the next hop's input token.
    fun setup_route(admin: &signer, n_positions: u64): (address, address, address) {
        let rt0 = funded_asset(admin, b"RT0");
        let rt1 = funded_asset(admin, b"RT1");
        let rt2 = funded_asset(admin, b"RT2");
        let rt3 = funded_asset(admin, b"RT3");
        (
            seeded_pool(admin, rt0, rt1, n_positions, MID_DENSITY),
            seeded_pool(admin, rt1, rt2, n_positions, MID_DENSITY),
            seeded_pool(admin, rt2, rt3, n_positions, MID_DENSITY)
        )
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

        let balance_before = clmm_assets::balance(@bench, clmm_assets::metadata(@bench, b"TKA"));
        clmm_pool::collect(admin, pool_id, -6000, 6000, owed_a, owed_b);
        let balance_after = clmm_assets::balance(@bench, clmm_assets::metadata(@bench, b"TKA"));
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
        clmm_pool::seed_positions(admin, pool_id, 8, MID_DENSITY, 42);
        // Jittered ranges around tick 0 leave the pool with liquidity in force
        // at some of them.
        let (a, b) = clmm_pool::vault_balances(pool_id);
        assert!(a > 0 || b > 0, 0);
    }

    #[test(admin = @bench)]
    fun test_seed_positions_scales_with_the_knob(admin: &signer) {
        let pool_id = setup(admin);
        clmm_pool::seed_positions(admin, pool_id, 64, MID_DENSITY, 7);
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
        clmm_pool::seed_positions(admin, pool_id, 16, MID_DENSITY, 2024);

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

    //
    // Routed swaps.
    //

    // A two-hop route sells the first pool's token A, then sells everything it
    // received into the second pool.
    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_multi_hop_two_pools(admin: &signer, alice: &signer) {
        setup_route(admin, 4);
        let legs = clmm_pool::route(alice, vector[0u64, 1], ROUTE_AMOUNT);
        assert!(vector::length(&legs) == 4, 0);
        assert!(*vector::borrow(&legs, 0) == ROUTE_AMOUNT, 0);
        assert!(*vector::borrow(&legs, 1) == *vector::borrow(&legs, 2), 0);
        assert!(*vector::borrow(&legs, 3) > 0, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_multi_hop_three_pools(admin: &signer, alice: &signer) {
        setup_route(admin, 4);
        let legs = clmm_pool::route(alice, vector[0u64, 1, 2], ROUTE_AMOUNT);
        assert!(vector::length(&legs) == 6, 0);
        assert!(*vector::borrow(&legs, 0) == ROUTE_AMOUNT, 0);
        assert!(*vector::borrow(&legs, 1) == *vector::borrow(&legs, 2), 0);
        assert!(*vector::borrow(&legs, 3) == *vector::borrow(&legs, 4), 0);
        assert!(*vector::borrow(&legs, 5) > 0, 0);
    }

    // Walking back down the chain has to open on the other side of the first
    // pool, which is the only way the second hop sees a token it holds.
    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_multi_hop_walks_the_chain_backwards(admin: &signer, alice: &signer) {
        setup_route(admin, 4);
        let legs = clmm_pool::route(alice, vector[2u64, 1, 0], ROUTE_AMOUNT);
        assert!(vector::length(&legs) == 6, 0);
        assert!(*vector::borrow(&legs, 0) == ROUTE_AMOUNT, 0);
        assert!(*vector::borrow(&legs, 1) == *vector::borrow(&legs, 2), 0);
        assert!(*vector::borrow(&legs, 3) == *vector::borrow(&legs, 4), 0);
        assert!(*vector::borrow(&legs, 5) > 0, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_multi_hop_tolerates_empty_path(admin: &signer, alice: &signer) {
        setup_route(admin, 4);
        let legs = clmm_pool::route(alice, vector[], ROUTE_AMOUNT);
        assert!(vector::is_empty(&legs), 0);
        clmm_pool::bench_swap_multi_hop(alice, vector[], ROUTE_AMOUNT);
    }

    // Three pools, so 3 and 4 name the same hops 0 and 1 do.
    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_multi_hop_tolerates_out_of_range_pool(admin: &signer, alice: &signer) {
        setup_route(admin, 4);
        let legs = clmm_pool::route(alice, vector[3u64, 4], ROUTE_AMOUNT);
        assert!(vector::length(&legs) == 4, 0);
        assert!(*vector::borrow(&legs, 0) == ROUTE_AMOUNT, 0);
        assert!(*vector::borrow(&legs, 1) == *vector::borrow(&legs, 2), 0);
    }

    // An amount the pool cannot absorb is cut to a slice of the reserve, and
    // the route carries on with what that slice produced.
    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_multi_hop_tolerates_amount_past_reserve(admin: &signer, alice: &signer) {
        let (first, _, _) = setup_route(admin, 4);
        let (reserve_a, _) = clmm_pool::vault_balances(first);
        let legs = clmm_pool::route(alice, vector[0u64, 1], OVERSIZED_AMOUNT);
        assert!(vector::length(&legs) == 4, 0);
        assert!(*vector::borrow(&legs, 0) == reserve_a / 20, 0);
        assert!(*vector::borrow(&legs, 0) < OVERSIZED_AMOUNT, 0);
        assert!(*vector::borrow(&legs, 3) > 0, 0);
    }

    //
    // Tick density.
    //

    // The knob's whole purpose: the same trade against the same liquidity
    // The harness backstops a pool far wider than anything else in the package
    // opens a position over, so check the fixed-point math holds out there and
    // that the publisher can afford it.
    #[test(admin = @bench)]
    fun test_wide_backstop_takes_a_swap(admin: &signer) {
        let (token_a, token_b) = assets(admin);
        clmm_pool::create_pool(
            admin, token_a, token_b, FEE_RATE, TICK_SPACING, clmm_tick_math::q96()
        );
        let pool_id = clmm_pool::pool_address(@bench, token_a, token_b, FEE_RATE, TICK_SPACING);
        let backstop = clmm_pool::bench_backstop_tick();
        assert!(backstop % (TICK_SPACING as i32) == 0, 0);
        assert!(backstop < clmm_tick_math::max_tick(), 0);

        clmm_pool::mint(admin, pool_id, -backstop, backstop, BASE_LIQUIDITY);
        assert!(clmm_pool::liquidity(pool_id) == BASE_LIQUIDITY, 0);

        // At tick zero a full-range position costs just under the liquidity
        // itself on each side, which is what the publisher has to be funded
        // for.
        let (reserve_a, reserve_b) = clmm_pool::vault_balances(pool_id);
        assert!((reserve_a as u128) <= BASE_LIQUIDITY, 0);
        assert!((reserve_b as u128) <= BASE_LIQUIDITY, 0);
        assert!((reserve_a as u128) * 2 > BASE_LIQUIDITY, 0);
        assert!((reserve_b as u128) * 2 > BASE_LIQUIDITY, 0);

        // The harness funds the publisher `FUNDING` per token and puts two
        // backstops on the tokens in the middle of the chain, so the bill has
        // to clear twice over with room to spare.
        assert!((reserve_a as u128) * 4 < (FUNDING as u128), 0);
        assert!((reserve_b as u128) * 4 < (FUNDING as u128), 0);

        clmm_pool::bench_swap_in(admin, pool_id, true, clmm_pool::bench_swap_size());
        assert!(clmm_pool::current_tick(pool_id) < 0, 0);
        assert!(clmm_pool::liquidity(pool_id) == BASE_LIQUIDITY, 0);
    }

    // The mix picks swap directions at random, so a price can wander as far as
    // the bound. Getting there and trading on past it must not abort: the
    // harness treats an abort as a crash, not as a trade that did nothing.
    //
    // An unseeded pool is the case to test, because it has no band to be
    // steered back into and so is the only one that reaches the bound at all.
    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_bench_swap_in_never_aborts_at_the_bound(admin: &signer, alice: &signer) {
        let pool_id = seeded_pool(
            admin, funded_asset(admin, b"DR0"), funded_asset(admin, b"DR1"), 0, HIGH_DENSITY
        );
        let (band_lower, band_upper) = clmm_pool::seeded_band(pool_id);
        assert!(band_lower == band_upper, 0);

        let i = 0;
        while (i < 4) {
            clmm_pool::bench_swap_in(alice, pool_id, true, DRAINING_TRADE);
            i = i + 1;
        };
        assert!(clmm_pool::sqrt_price(pool_id) == clmm_tick_math::min_sqrt_price() + 1, 0);

        // Resting on the bound, a swap that way moves nothing rather than
        // failing.
        let (before_a, before_b) = clmm_pool::vault_balances(pool_id);
        clmm_pool::bench_swap_in(alice, pool_id, true, DRAINING_TRADE);
        let (after_a, after_b) = clmm_pool::vault_balances(pool_id);
        assert!(after_a == before_a && after_b == before_b, 0);

        // A zero-sized trade is a no-op too, in either direction.
        clmm_pool::bench_swap_in(alice, pool_id, true, 0);
        clmm_pool::bench_swap_in(alice, pool_id, false, 0);

        // And the pool is empty one way, not stuck: the price comes back.
        clmm_pool::bench_swap_in(alice, pool_id, false, DRAINING_TRADE);
        assert!(clmm_pool::sqrt_price(pool_id) > clmm_tick_math::min_sqrt_price() + 1, 0);
    }

    // Direction is a hint, not an order: a price that has left the seeded band
    // trades back toward it. Without this the mix's walk leaves the lattice for
    // good after a handful of trades, and every swap after that crosses only
    // the backstop, which is the measurement quietly going to zero.
    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_swap_direction_reverts_to_the_band(admin: &signer, alice: &signer) {
        let pool_id = seeded_pool(
            admin,
            funded_asset(admin, b"MR0"),
            funded_asset(admin, b"MR1"),
            DENSITY_SEED_POSITIONS,
            HIGH_DENSITY
        );
        let (band_lower, band_upper) = clmm_pool::seeded_band(pool_id);
        assert!(band_lower < band_upper, 0);

        // The first trade starts at the center, so how far it reaches is how
        // far one trade can take a price that is still inside the band.
        let trade = clmm_pool::bench_swap_size();
        clmm_pool::bench_swap_in(alice, pool_id, true, trade);
        let span = 0 - clmm_pool::current_tick(pool_id);
        assert!(span > 0, 0);

        // The mix flips a coin for the direction, so walk one here too.
        let state = 987654321u64;
        let crossings = 0;
        let i = 0;
        while (i < WALK_SWAPS) {
            state = (state * LCG_MUL + LCG_INC) % LCG_MOD;
            clmm_pool::bench_swap_in(alice, pool_id, (state / 65536) % 2 == 0, trade);
            let tick = clmm_pool::current_tick(pool_id);
            assert!(tick > band_lower - span, 0);
            assert!(tick < band_upper + span, 0);
            crossings = crossings + clmm_pool::last_swap_crossings(pool_id);
            i = i + 1;
        };

        // A walk that left the lattice would still swap, just against the
        // backstop alone, so the crossing count is what catches it.
        assert!(crossings >= WALK_SWAPS * WALK_MIN_CROSSINGS, 0);
    }

    // crosses far more initialized ticks when the seeded lattice is tighter.
    #[test(admin = @bench)]
    fun test_tick_density_changes_crossings(admin: &signer) {
        let sparse = seeded_pool(
            admin,
            funded_asset(admin, b"LD0"),
            funded_asset(admin, b"LD1"),
            DENSITY_SEED_POSITIONS,
            LOW_DENSITY
        );
        let dense = seeded_pool(
            admin,
            funded_asset(admin, b"HD0"),
            funded_asset(admin, b"HD1"),
            DENSITY_SEED_POSITIONS,
            HIGH_DENSITY
        );
        assert!(clmm_pool::tick_density(sparse) == LOW_DENSITY, 0);
        assert!(clmm_pool::tick_density(dense) == HIGH_DENSITY, 0);

        // The counts below are the shipped configuration's, so retuning the
        // knob has to come back through here.
        assert!(clmm_pool::bench_tick_density() == HIGH_DENSITY, 0);

        // The harness's own largest trade, so lowering it below the point
        // where the knob does anything fails here.
        let trade = clmm_pool::bench_swap_size();
        clmm_pool::bench_swap_in(admin, sparse, true, trade);
        clmm_pool::bench_swap_in(admin, dense, true, trade);

        // Both pools end at the same price, so the difference is the lattice
        // and nothing else.
        assert!(clmm_pool::current_tick(sparse) == clmm_pool::current_tick(dense), 0);
        let sparse_crossings = clmm_pool::last_swap_crossings(sparse);
        let dense_crossings = clmm_pool::last_swap_crossings(dense);
        assert!(sparse_crossings <= 3, 0);
        assert!(dense_crossings >= 18, 0);
        assert!(dense_crossings >= 5 * sparse_crossings, 0);
    }

    /// Crossings over `WALK_SWAPS` coin-flipped trades, the same flips each
    /// time so two pools can be compared against one another.
    fun walk_crossings(trader: &signer, pool_id: address): u64 {
        let trade = clmm_pool::bench_swap_size();
        let state = 987654321u64;
        let crossings = 0;
        let i = 0;
        while (i < WALK_SWAPS) {
            state = (state * LCG_MUL + LCG_INC) % LCG_MOD;
            clmm_pool::bench_swap_in(trader, pool_id, (state / 65536) % 2 == 0, trade);
            crossings = crossings + clmm_pool::last_swap_crossings(pool_id);
            i = i + 1;
        };
        crossings
    }

    // The knob has to keep separating for a whole run, not just for the first
    // trade. A price left to drift ends up alone with the backstop, where both
    // densities cross the same nothing.
    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_tick_density_separates_after_a_walk(admin: &signer, alice: &signer) {
        let sparse = seeded_pool(
            admin,
            funded_asset(admin, b"WS0"),
            funded_asset(admin, b"WS1"),
            DENSITY_SEED_POSITIONS,
            LOW_DENSITY
        );
        let dense = seeded_pool(
            admin,
            funded_asset(admin, b"WD0"),
            funded_asset(admin, b"WD1"),
            DENSITY_SEED_POSITIONS,
            HIGH_DENSITY
        );
        let sparse_crossings = walk_crossings(alice, sparse);
        let dense_crossings = walk_crossings(alice, dense);
        assert!(dense_crossings >= WALK_SWAPS * WALK_MIN_CROSSINGS, 0);
        assert!(dense_crossings >= 5 * sparse_crossings, 0);
    }

    // A route's hops take whichever direction the token they carry forces, so
    // only the single swaps are steered back to the band. They outnumber routes
    // forty to fifteen in the mix, which has to be enough to hold it.
    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_routes_do_not_undo_the_band(admin: &signer, alice: &signer) {
        let rt0 = funded_asset(admin, b"MX0");
        let rt1 = funded_asset(admin, b"MX1");
        let rt2 = funded_asset(admin, b"MX2");
        let rt3 = funded_asset(admin, b"MX3");
        let pool_id = seeded_pool(admin, rt0, rt1, DENSITY_SEED_POSITIONS, HIGH_DENSITY);
        seeded_pool(admin, rt1, rt2, DENSITY_SEED_POSITIONS, HIGH_DENSITY);
        seeded_pool(admin, rt2, rt3, DENSITY_SEED_POSITIONS, HIGH_DENSITY);

        let trade = clmm_pool::bench_swap_size();
        let state = 987654321u64;
        let crossings = 0;
        let swaps = 0;
        let i = 0;
        while (i < WALK_SWAPS) {
            state = (state * LCG_MUL + LCG_INC) % LCG_MOD;
            let roll = (state / 65536) % 55;
            if (roll < 40) {
                clmm_pool::bench_swap_in(alice, pool_id, roll % 2 == 0, trade);
                crossings = crossings + clmm_pool::last_swap_crossings(pool_id);
                swaps = swaps + 1;
            } else if (roll % 2 == 0) {
                clmm_pool::bench_swap_multi_hop(alice, vector[0u64, 1, 2], trade);
            } else {
                clmm_pool::bench_swap_multi_hop(alice, vector[2u64, 1, 0], trade);
            };
            i = i + 1;
        };
        assert!(swaps > 0, 0);
        assert!(crossings >= swaps * WALK_MIN_CROSSINGS, 0);
    }

    // The recipe a transaction harness follows: seed liquidity once as admin,
    // onboard each account with one transaction, then run the mix. Every step
    // after onboarding must succeed from a plain signer holding nothing.
    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_harness_onboard_then_mix(admin: &signer, alice: &signer) {
        // Three chained pools, each backstopped by a range wider than the mix
        // can push the price, so no swap ever finds an empty book.
        let (pool_id, _, _) = setup_route(admin, 8);

        clmm_pool::bench_onboard(alice, pool_id, -1200, 1200, EXTRA_LIQUIDITY, FUNDING);
        assert!(
            clmm_pool::position_liquidity(pool_id, @0xa11ce, -1200, 1200)
                == EXTRA_LIQUIDITY,
            0
        );

        clmm_pool::bench_swap_in(alice, pool_id, true, 1000000);
        clmm_pool::bench_swap_in(alice, pool_id, false, 1000000);
        clmm_pool::bench_swap_multi_hop(alice, vector[0u64, 1, 2], ROUTE_AMOUNT);
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
