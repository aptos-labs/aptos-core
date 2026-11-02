#[test_only]
module bench::stableswap_tests {
    use std::vector;
    use aptos_framework::timestamp;
    use bench::ss_amp;
    use bench::ss_assets;
    use bench::ss_lp;
    use bench::ss_math;
    use bench::ss_pool;

    /// The generator's default configuration, mirrored here so a change on
    /// either side shows up as a failing test rather than a failed run.
    const AMP_2COIN: u64 = 200;
    const AMP_3COIN: u64 = 600;
    /// A softened pool's amplification: the configured one over 100, floored
    /// at 2.
    const AMP_2COIN_SOFT: u64 = 2;
    const AMP_3COIN_SOFT: u64 = 6;
    const FEE_BPS: u64 = 4;
    const IMBALANCE_BP: u64 = 2000;
    const SWAP_UNITS: u64 = 1000;
    const MATH_ONLY_POOLS: u64 = 8;
    const MATH_ONLY_A: u64 = 200;
    const RAMP_TARGET_A: u64 = 300;
    const RAMP_DURATION_SECS: u64 = 86400;
    const SEED_UNITS: u64 = 20000;
    const ONBOARD_FUND_UNITS: u64 = 100000;
    const ONBOARD_DEPOSIT_UNITS: u64 = 20;
    const ADD_UNITS: u64 = 10;
    const REMOVE_BPS: u64 = 2000;

    /// Each pool's target composition, in basis points of its lead coin.
    const SKEW_POOL_1: u64 = 10000;
    const SKEW_POOL_2: u64 = 1000;
    const SKEW_POOL_3: u64 = 300;
    const SKEW_POOL_4: u64 = 100;

    const ALICE: address = @0xa11ce;
    const BOB: address = @0xb0b;

    /// Scaled balance of a million tokens at the common precision.
    const XP: u256 = 10000000000000000;

    fun setup(aptos_framework: &signer, admin: &signer) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        ss_assets::create_asset_entry(admin, b"SS0", 6);
        ss_assets::create_asset_entry(admin, b"SS1", 6);
        ss_assets::create_asset_entry(admin, b"SS2", 6);
        ss_assets::create_asset_entry(admin, b"SS3", 8);
        ss_assets::create_asset_entry(admin, b"SS4", 8);
        ss_pool::initialize(admin);
        // The generator's four-pool ladder: two at the configured
        // amplification and two at a hundredth of it, at compositions spanning
        // two orders of magnitude.
        ss_pool::create_pool(
            admin, 1, vector[b"SS0", b"SS1"], AMP_2COIN, FEE_BPS, SKEW_POOL_1);
        ss_pool::create_pool(
            admin,
            2,
            vector[b"SS2", b"SS3", b"SS4"],
            AMP_3COIN,
            FEE_BPS,
            SKEW_POOL_2,
        );
        ss_pool::create_pool(
            admin,
            3,
            vector[b"SS1", b"SS3"],
            AMP_2COIN_SOFT,
            FEE_BPS,
            SKEW_POOL_3,
        );
        ss_pool::create_pool(
            admin,
            4,
            vector[b"SS0", b"SS2", b"SS4"],
            AMP_3COIN_SOFT,
            FEE_BPS,
            SKEW_POOL_4,
        );
        ss_pool::seed_liquidity(admin, 1, SEED_UNITS);
        ss_pool::seed_liquidity(admin, 2, SEED_UNITS);
        ss_pool::seed_liquidity(admin, 3, SEED_UNITS);
        ss_pool::seed_liquidity(admin, 4, SEED_UNITS);
        ss_amp::set_ramp(admin, 2, RAMP_TARGET_A, RAMP_DURATION_SECS);
    }

    fun assert_reserves_non_zero() {
        let pool_id = 1;
        while (pool_id <= 4) {
            let n = ss_pool::num_coins(pool_id);
            let k = 0;
            while (k < n) {
                assert!(ss_pool::reserve(pool_id, k) > 0, 0);
                k = k + 1;
            };
            pool_id = pool_id + 1;
        }
    }

    /// A pool's reserves lifted to the common precision, read back through the
    /// same views the generator has.
    fun pool_xp(pool_id: u64): vector<u256> {
        let n = ss_pool::num_coins(pool_id);
        let balances = vector::empty<u64>();
        let rates = vector::empty<u256>();
        let k = 0;
        while (k < n) {
            vector::push_back(&mut balances, ss_pool::reserve(pool_id, k));
            vector::push_back(
                &mut rates,
                ss_math::rate_for_decimals(
                    ss_assets::decimals(ss_pool::coin_at(pool_id, k))),
            );
            k = k + 1;
        };
        ss_math::xp_mem(&balances, &rates)
    }

    /// Rounds `get_D` takes on a pool as it currently stands, at the same
    /// amplification a swap on it would use.
    fun pool_d_iters(pool_id: u64): u64 {
        let xp = pool_xp(pool_id);
        let (_, iters) = ss_math::get_D_with_iters(&xp, ss_amp::amp(pool_id));
        iters
    }

    #[test(aptos_framework = @0x1, admin = @bench, alice = @0xa11ce, bob = @0xb0b)]
    fun test_harness_onboard_then_mix(
        aptos_framework: &signer,
        admin: &signer,
        alice: &signer,
        bob: &signer,
    ) {
        setup(aptos_framework, admin);
        assert!(ss_pool::num_pools() == 4, 0);
        assert!(ss_pool::num_coins(1) == 2, 0);
        assert!(ss_pool::num_coins(2) == 3, 0);
        assert!(ss_pool::num_coins(3) == 2, 0);
        assert!(ss_pool::num_coins(4) == 3, 0);
        assert_reserves_non_zero();

        ss_pool::bench_onboard(alice, ONBOARD_FUND_UNITS, ONBOARD_DEPOSIT_UNITS);
        ss_pool::bench_onboard(bob, ONBOARD_FUND_UNITS, ONBOARD_DEPOSIT_UNITS);
        assert!(ss_pool::lp_balance(ALICE, 1) > 0, 0);
        assert!(ss_pool::lp_balance(ALICE, 2) > 0, 0);
        assert!(ss_pool::lp_balance(BOB, 1) > 0, 0);

        // One call of every mix branch, at the generator's arguments.
        ss_pool::bench_swap(alice, 1, 0, 1, SWAP_UNITS);
        ss_pool::bench_swap(bob, 2, 0, 2, SWAP_UNITS);
        ss_pool::bench_swap(alice, 3, 1, 0, SWAP_UNITS);
        ss_pool::bench_swap(bob, 4, 2, 0, SWAP_UNITS);
        ss_lp::bench_add_imbalanced(alice, 1, ADD_UNITS, IMBALANCE_BP);
        ss_lp::bench_add_imbalanced(bob, 2, ADD_UNITS, IMBALANCE_BP);
        ss_lp::bench_remove_one(alice, 1, 0, REMOVE_BPS);
        ss_lp::bench_remove_one(bob, 2, 2, REMOVE_BPS);
        ss_math::bench_math_only(alice, MATH_ONLY_POOLS, MATH_ONLY_A, 42);
        ss_amp::bench_ramp(bob, 2, RAMP_TARGET_A, RAMP_DURATION_SECS);

        assert_reserves_non_zero();
        assert!(ss_pool::lp_supply(1) > 0, 0);
        assert!(ss_pool::lp_supply(2) > 0, 0);
        assert!(ss_pool::lp_balance(ALICE, 1) <= ss_pool::lp_supply(1), 0);
        assert!(ss_pool::lp_balance(BOB, 2) <= ss_pool::lp_supply(2), 0);
        // A swap pays out, so the taker ends up holding more than it was
        // funded with on one side and the pool keeps the rest.
        assert!(ss_assets::primary_balance(ALICE, ss_pool::coin_at(1, 1)) > 0, 0);
    }

    #[test(aptos_framework = @0x1, admin = @bench, alice = @0xa11ce)]
    fun test_swap_debits_the_traders_own_balance(
        aptos_framework: &signer, admin: &signer, alice: &signer
    ) {
        setup(aptos_framework, admin);
        ss_pool::bench_onboard(alice, ONBOARD_FUND_UNITS, ONBOARD_DEPOSIT_UNITS);

        let coin_in = ss_pool::coin_at(1, 0);
        let held_before = ss_assets::primary_balance(ALICE, coin_in);
        let reserve_before = ss_pool::reserve(1, 0);
        ss_pool::bench_swap(alice, 1, 0, 1, SWAP_UNITS);

        // What left the trader is exactly what reached the pool. A funded
        // trader mints nothing, so the trade is a plain store debit.
        let paid = held_before - ss_assets::primary_balance(ALICE, coin_in);
        assert!(paid > 0, 0);
        assert!(ss_pool::reserve(1, 0) - reserve_before == paid, 0);
    }

    #[test(aptos_framework = @0x1, admin = @bench, alice = @0xa11ce, bob = @0xb0b)]
    fun test_swap_tops_up_only_a_trader_that_is_short(
        aptos_framework: &signer,
        admin: &signer,
        alice: &signer,
        bob: &signer,
    ) {
        setup(aptos_framework, admin);
        // Bob never onboarded, so he holds nothing of the input coin.
        let coin_in = ss_pool::coin_at(1, 0);
        assert!(ss_assets::primary_balance(BOB, coin_in) == 0, 0);

        let reserve_before = ss_pool::reserve(1, 0);
        ss_pool::bench_swap(bob, 1, 0, 1, SWAP_UNITS);
        assert!(ss_pool::reserve(1, 0) > reserve_before, 0);

        // The top-up overshoots what the trade needed, so the next trade is
        // paid for out of the balance and mints nothing.
        let after_first = ss_assets::primary_balance(BOB, coin_in);
        assert!(after_first > 0, 0);
        ss_pool::bench_swap(bob, 1, 0, 1, SWAP_UNITS);
        assert!(ss_assets::primary_balance(BOB, coin_in) < after_first, 0);

        // An onboarded trader is never short to begin with.
        ss_pool::bench_onboard(alice, ONBOARD_FUND_UNITS, ONBOARD_DEPOSIT_UNITS);
        let held = ss_assets::primary_balance(ALICE, coin_in);
        ss_pool::bench_swap(alice, 1, 0, 1, SWAP_UNITS);
        assert!(ss_assets::primary_balance(ALICE, coin_in) < held, 0);
    }

    #[test(aptos_framework = @0x1, admin = @bench, alice = @0xa11ce)]
    fun test_swap_cap_binds_and_the_trader_pays_it(
        aptos_framework: &signer, admin: &signer, alice: &signer
    ) {
        setup(aptos_framework, admin);
        ss_pool::bench_onboard(alice, ONBOARD_FUND_UNITS, ONBOARD_DEPOSIT_UNITS);

        let coin_in = ss_pool::coin_at(1, 0);
        let held_before = ss_assets::primary_balance(ALICE, coin_in);
        let reserve_before = ss_pool::reserve(1, 0);
        // A thousand times the whole reserve. The cap is five percent of it,
        // and that is both what the pool takes and what the trader pays.
        ss_pool::bench_swap(alice, 1, 0, 1, 1000000000000);
        assert!(
            ss_pool::reserve(1, 0) - reserve_before == reserve_before / 20, 0);
        assert!(
            held_before - ss_assets::primary_balance(ALICE, coin_in)
                == reserve_before / 20,
            0,
        );
    }

    #[test(aptos_framework = @0x1, admin = @bench, alice = @0xa11ce)]
    fun test_swap_tolerates_input_larger_than_reserve(
        aptos_framework: &signer, admin: &signer, alice: &signer
    ) {
        setup(aptos_framework, admin);
        ss_pool::bench_onboard(alice, ONBOARD_FUND_UNITS, ONBOARD_DEPOSIT_UNITS);

        let before_in = ss_pool::reserve(1, 1);
        let before_out = ss_pool::reserve(1, 0);
        // A thousand times the whole reserve, on coin indices past the end
        // that are also equal to each other. They fold back to a swap of
        // coin 1 into coin 0.
        ss_pool::bench_swap(alice, 1, 7, 7, 1000000000000);
        ss_pool::bench_swap(alice, 2, 0, 1, 1000000000000);
        assert_reserves_non_zero();
        assert!(ss_pool::reserve(1, 1) > before_in, 0);
        assert!(ss_pool::reserve(1, 0) < before_out, 0);

        // A pool that was never created is a no-op, not an abort.
        ss_pool::bench_swap(alice, 99, 0, 1, SWAP_UNITS);
        assert_reserves_non_zero();
    }

    #[test(aptos_framework = @0x1, admin = @bench, alice = @0xa11ce, bob = @0xb0b)]
    fun test_remove_one_tolerates_more_lp_than_held(
        aptos_framework: &signer,
        admin: &signer,
        alice: &signer,
        bob: &signer,
    ) {
        setup(aptos_framework, admin);
        ss_pool::bench_onboard(alice, ONBOARD_FUND_UNITS, ONBOARD_DEPOSIT_UNITS);

        // Ten times the whole position leaves the caller with nothing, not an
        // abort.
        ss_lp::bench_remove_one(alice, 1, 0, 100000);
        assert!(ss_pool::lp_balance(ALICE, 1) == 0, 0);
        assert_reserves_non_zero();

        // A second attempt, and an attempt by an account that never
        // onboarded, both do nothing.
        ss_lp::bench_remove_one(alice, 1, 0, 100000);
        ss_lp::bench_remove_one(bob, 1, 0, REMOVE_BPS);
        assert!(ss_pool::lp_balance(BOB, 1) == 0, 0);
        assert_reserves_non_zero();
    }

    #[test(alice = @0xa11ce)]
    fun test_math_only_tolerates_untouched_storage(alice: &signer) {
        // No assets, no registry, and no clock: the math path must not read
        // any of them.
        ss_math::bench_math_only(alice, MATH_ONLY_POOLS, MATH_ONLY_A, 7);
        ss_math::bench_math_only(alice, 0, 0, 0);
        // Far past the cap on synthetic pools.
        ss_math::bench_math_only(alice, 1000000, 18446744073709551615, 3);
    }

    #[test]
    fun test_get_d_tolerates_iteration_cap() {
        let amp = ss_math::a_precision();
        // Twenty orders of magnitude apart, which is further than any pool
        // this package can reach. Both orderings, because the guard is on the
        // balance the loop is about to divide by and a small first balance
        // blows the running product up a round earlier than a large one.
        let skewed = vector[1u256, 100000000000000000000u256];
        let (d, iters) = ss_math::get_D_with_iters(&skewed, amp);
        assert!(iters <= ss_math::max_iters(), 0);
        assert!(d > 0, 0);

        let skewed_reversed = vector[100000000000000000000u256, 1u256];
        let (d, iters) = ss_math::get_D_with_iters(&skewed_reversed, amp);
        assert!(iters <= ss_math::max_iters(), 0);
        assert!(d > 0, 0);

        // Balances large enough that the products would leave a u256. The
        // loop has to bail out rather than overflow, whichever end the large
        // balance is at.
        let max = 115792089237316195423570985008687907853269984665640564039457584007913129639935u256;
        let (d, iters) = ss_math::get_D_with_iters(&vector[1u256, max], amp);
        assert!(iters <= ss_math::max_iters(), 0);
        assert!(d > 0, 0);

        let (d, iters) = ss_math::get_D_with_iters(&vector[max, 1u256], amp);
        assert!(iters <= ss_math::max_iters(), 0);
        assert!(d > 0, 0);

        // Three coins, so the large balance is neither first nor last.
        let (d, iters) =
            ss_math::get_D_with_iters(&vector[1u256, max, 1u256], amp);
        assert!(iters <= ss_math::max_iters(), 0);
        assert!(d > 0, 0);

        let (y, iters) = ss_math::get_y_with_iters(0, 1, XP * 2, &skewed, amp);
        assert!(iters <= ss_math::max_iters(), 0);
        assert!(y > 0, 0);
    }

    // The ladder exists to keep `get_D` off a single trip count. Retuning the
    // pools' amplifications, compositions or seed size back toward balance
    // collapses the counts to three and fails here.
    #[test(aptos_framework = @0x1, admin = @bench, alice = @0xa11ce, bob = @0xb0b)]
    fun test_pool_ladder_spreads_get_d_trip_counts(
        aptos_framework: &signer,
        admin: &signer,
        alice: &signer,
        bob: &signer,
    ) {
        setup(aptos_framework, admin);

        // Seeded states. A balanced pool solves in one round; the ladder's
        // other three cost progressively more.
        assert!(pool_d_iters(1) == 1, 0);
        assert!(pool_d_iters(2) == 3, 0);
        assert!(pool_d_iters(3) == 6, 0);
        assert!(pool_d_iters(4) == 9, 0);

        // Under traffic. One swap per pool per account, which is a short run
        // rather than the mix's millions, so the pools move off their seeded
        // states without the test running out of gas.
        ss_pool::bench_onboard(alice, ONBOARD_FUND_UNITS, ONBOARD_DEPOSIT_UNITS);
        ss_pool::bench_onboard(bob, ONBOARD_FUND_UNITS, ONBOARD_DEPOSIT_UNITS);
        ss_pool::bench_swap(alice, 1, 0, 1, SWAP_UNITS);
        ss_pool::bench_swap(bob, 1, 1, 0, SWAP_UNITS);
        ss_pool::bench_swap(alice, 2, 0, 1, SWAP_UNITS);
        ss_pool::bench_swap(bob, 2, 2, 0, SWAP_UNITS);
        ss_pool::bench_swap(alice, 3, 0, 1, SWAP_UNITS);
        ss_pool::bench_swap(bob, 3, 1, 0, SWAP_UNITS);
        ss_pool::bench_swap(alice, 4, 0, 2, SWAP_UNITS);
        ss_pool::bench_swap(bob, 4, 2, 1, SWAP_UNITS);

        // Traffic must not average the ladder away: the softened, lopsided
        // pools still cost more rounds than the balanced one, and the spread
        // stays wide enough that no single count dominates the mix.
        let low = pool_d_iters(1);
        let high = pool_d_iters(4);
        assert!(high > low, 0);
        assert!(high - low >= 3, 0);
        assert!(pool_d_iters(3) > low, 0);
        assert!(low > 1, 0);
    }

    #[test]
    fun test_get_d_trip_count_differs_between_balanced_and_imbalanced() {
        let amp = (AMP_3COIN as u256) * ss_math::a_precision();
        let balanced = vector[XP, XP, XP];
        let (d_balanced, balanced_iters) =
            ss_math::get_D_with_iters(&balanced, amp);
        // A perfectly balanced pool solves in one round: D is the sum.
        assert!(d_balanced == XP * 3, 0);
        assert!(balanced_iters == 1, 0);

        let imbalanced = vector[XP / 10, XP, XP * 4];
        let (d_imbalanced, imbalanced_iters) =
            ss_math::get_D_with_iters(&imbalanced, amp);
        assert!(imbalanced_iters > balanced_iters, 0);
        assert!(d_imbalanced < XP * 5 + XP / 10, 0);

        // The same balances at a hundredth of the amplification cost more
        // rounds still, which is what the ladder's softened pools buy.
        let soft = (AMP_3COIN_SOFT as u256) * ss_math::a_precision();
        let (_, soft_iters) = ss_math::get_D_with_iters(&imbalanced, soft);
        assert!(soft_iters > imbalanced_iters, 0);
    }

    #[test]
    fun test_get_d_matches_hand_computed_values() {
        let amp = ss_math::a_precision();
        // At the solution, D + D^3 / (n^n * x0 * x1) = A * n * S. For A = 1
        // and balances of 100 and 400 that is D + D^3 / 160000 = 1000, whose
        // root rounds to 446.
        let pool = vector[100u256, 400u256];
        assert!(ss_math::get_D(&pool, amp) == 446, 0);

        // The invariant is homogeneous, so ten times every balance is ten
        // times D, up to the rounding the integer loop introduces.
        let scaled = vector[1000u256, 4000u256];
        assert!(ss_math::get_D(&scaled, amp) == 4459, 0);

        // An empty pool has no invariant to solve.
        let empty = vector[0u256, 0u256];
        assert!(ss_math::get_D(&empty, amp) == 0, 0);
    }

    #[test]
    fun test_get_y_returns_the_balance_it_was_given() {
        let amp = (AMP_2COIN as u256) * ss_math::a_precision();
        let pool = vector[XP, XP];
        // Solving for coin 1 while coin 0 is left where it already is has to
        // land back on coin 1's own balance.
        let y = ss_math::get_y(0, 1, XP, &pool, amp);
        assert!(y >= XP - 2 && y <= XP + 2, 0);

        // More of coin 0 means less of coin 1, and by less than the amount
        // put in, because the curve charges slippage.
        let y_after = ss_math::get_y(0, 1, XP + XP / 100, &pool, amp);
        assert!(y_after < XP, 0);
        assert!(y_after > XP - XP / 100, 0);
    }

    #[test]
    fun test_get_y_d_pays_less_as_the_invariant_shrinks() {
        let amp = (AMP_3COIN as u256) * ss_math::a_precision();
        let pool = vector[XP, XP, XP];
        let d = ss_math::get_D(&pool, amp);
        // Driving D down by a tenth has to leave coin 0 below where it was,
        // and driving it down further has to leave it lower still.
        let y_small = ss_math::get_y_D(0, &pool, amp, d - d / 10);
        let y_smaller = ss_math::get_y_D(0, &pool, amp, d - d / 5);
        assert!(y_small < XP, 0);
        assert!(y_smaller < y_small, 0);
        assert!(ss_math::get_y_D(0, &pool, amp, d) >= XP - 2, 0);
    }

    #[test]
    fun test_rate_and_unit_scales_follow_decimals() {
        assert!(ss_math::rate_for_decimals(6) == 10000, 0);
        assert!(ss_math::rate_for_decimals(8) == 100, 0);
        assert!(ss_math::rate_for_decimals(10) == 1, 0);
        // Past the common precision there is nothing left to scale up.
        assert!(ss_math::rate_for_decimals(18) == 1, 0);
        assert!(ss_math::unit_for_decimals(6) == 1000000, 0);
        assert!(ss_math::unit_for_decimals(8) == 100000000, 0);
    }

    #[test]
    fun test_xp_mem_lifts_mixed_decimals_to_one_scale() {
        let balances = vector[1000000u64, 100000000u64];
        let rates = vector[
            ss_math::rate_for_decimals(6),
            ss_math::rate_for_decimals(8),
        ];
        let xp = ss_math::xp_mem(&balances, &rates);
        // One whole token of each coin, so both land on the same number.
        assert!(*vector::borrow(&xp, 0) == *vector::borrow(&xp, 1), 0);
        assert!(ss_math::sum(&xp) == 20000000000, 0);

        // A zero balance becomes one, so the Newton loops never divide by it.
        let empty = vector[0u64];
        let one = vector[1u256];
        assert!(*vector::borrow(&ss_math::xp_mem(&empty, &one), 0) == 1, 0);
    }

    #[test(aptos_framework = @0x1, admin = @bench, alice = @0xa11ce)]
    fun test_ramp_moves_amp_over_time(
        aptos_framework: &signer, admin: &signer, alice: &signer
    ) {
        setup(aptos_framework, admin);
        let start = ss_amp::amp(2);
        assert!(start == (AMP_3COIN as u256) * ss_math::a_precision(), 0);

        timestamp::fast_forward_seconds(RAMP_DURATION_SECS / 2);
        let midway = ss_amp::amp(2);
        assert!(midway < start, 0);
        assert!(midway > (RAMP_TARGET_A as u256) * ss_math::a_precision(), 0);

        timestamp::fast_forward_seconds(RAMP_DURATION_SECS);
        assert!(
            ss_amp::amp(2) == (RAMP_TARGET_A as u256) * ss_math::a_precision(),
            0,
        );

        // The unpermissioned ramp clamps rather than rejecting, and a pool
        // that does not exist is a no-op.
        ss_amp::bench_ramp(alice, 2, 0, 0);
        ss_amp::bench_ramp(alice, 99, RAMP_TARGET_A, RAMP_DURATION_SECS);
        assert!(ss_amp::amp(2) > 0, 0);
    }

    #[test(aptos_framework = @0x1, admin = @bench, alice = @0xa11ce)]
    fun test_add_imbalanced_tolerates_extreme_arguments(
        aptos_framework: &signer, admin: &signer, alice: &signer
    ) {
        setup(aptos_framework, admin);
        ss_pool::bench_onboard(alice, ONBOARD_FUND_UNITS, ONBOARD_DEPOSIT_UNITS);

        let before = ss_pool::lp_balance(ALICE, 2);
        // A deposit past the cap and a skew past the ceiling.
        ss_lp::bench_add_imbalanced(alice, 2, 18446744073709551615, 999999);
        assert!(ss_pool::lp_balance(ALICE, 2) > before, 0);
        assert_reserves_non_zero();

        // Nothing to deposit and no such pool are both no-ops.
        ss_lp::bench_add_imbalanced(alice, 2, 0, IMBALANCE_BP);
        ss_lp::bench_add_imbalanced(alice, 99, ADD_UNITS, IMBALANCE_BP);
        assert_reserves_non_zero();
    }

    // A deposit may be more lopsided than the pool it lands in but never more
    // balanced, so the mix's own liquidity traffic cannot average a pool back
    // to balance and cheap solves.
    #[test(aptos_framework = @0x1, admin = @bench, alice = @0xa11ce)]
    fun test_add_imbalanced_cannot_balance_a_lopsided_pool(
        aptos_framework: &signer, admin: &signer, alice: &signer
    ) {
        setup(aptos_framework, admin);
        ss_pool::bench_onboard(alice, ONBOARD_FUND_UNITS, ONBOARD_DEPOSIT_UNITS);

        // Pool 4 sits at one percent. Asking for a balanced deposit into it
        // still deposits at one percent.
        let lead_before = ss_pool::reserve(4, 0);
        let other_before = ss_pool::reserve(4, 1);
        ss_lp::bench_add_imbalanced(alice, 4, 100000, 10000);
        let lead_added = ss_pool::reserve(4, 0) - lead_before;
        let other_added = ss_pool::reserve(4, 1) - other_before;
        assert!(lead_added > 0, 0);
        assert!(other_added * 50 < lead_added, 0);
    }

    #[test(aptos_framework = @0x1, admin = @bench, alice = @0xa11ce)]
    fun test_onboard_is_idempotent(
        aptos_framework: &signer, admin: &signer, alice: &signer
    ) {
        setup(aptos_framework, admin);
        ss_pool::bench_onboard(alice, ONBOARD_FUND_UNITS, ONBOARD_DEPOSIT_UNITS);
        let once = ss_pool::lp_balance(ALICE, 1);
        ss_pool::bench_onboard(alice, ONBOARD_FUND_UNITS, ONBOARD_DEPOSIT_UNITS);
        assert!(ss_pool::lp_balance(ALICE, 1) > once, 0);
        assert_reserves_non_zero();
    }
}
