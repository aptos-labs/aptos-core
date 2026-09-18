#[test_only]
module bench::dex_aggregator_tests {
    use std::signer;
    use std::vector;
    use bench::dexr_assets;
    use bench::dexr_backend;
    use bench::dexr_markers::{
        Book, Clmm, Cpmm, Stable,
        A0, A1, A2, A3, A4, A5, A6, A7,
        M0, M1, M2, M3, M4, M5, M6, M7, M8, M9, M10,
        M11, M12, M13, M14, M15, M16, M17, M18, M19, M20,
    };
    use bench::dexr_math;
    use bench::dexr_pool_book;
    use bench::dexr_pool_clmm;
    use bench::dexr_pool_cpmm;
    use bench::dexr_pool_stable;
    use bench::dexr_router;

    const N_ASSETS: u64 = 8;
    const DECIMALS: u8 = 6;

    /// Pool sizing. Small enough that the whole init sequence fits inside the
    /// unit test execution bound, large enough that a trade never clamps.
    const RESERVE: u64 = 1_000_000_000;
    const FEE_BPS: u64 = 30;
    const STABLE_FEE_BPS: u64 = 6;
    const AMP: u64 = 100;
    const TICKS: u64 = 8;
    const TICK_LIQUIDITY: u64 = 10_000;
    const DEPTH: u64 = 8;
    const LEVEL_SIZE: u64 = 10_000;
    const POOLS_PER_BACKEND: u64 = 4;

    const FUNDING: u64 = 1_000_000_000;
    const AMOUNT: u64 = 1_000_000;

    const U64_MAX: u64 = 18446744073709551615;

    /// Pools one `seed_pools` transaction creates, mirroring `SEED_BATCH` in
    /// the harness, at the shape the harness deploys: a twelve-figure reserve
    /// and a range of `tick_crossings` (eight) times `RANGE_SLACK` (four).
    const SEED_BATCH: u64 = 16;
    const DEPLOYED_RESERVE: u64 = 1_000_000_000_000;
    const DEPLOYED_RANGE: u64 = 32;
    const DEPLOYED_UNIT: u64 = 12_500_000;

    fun make_assets(admin: &signer): vector<address> {
        let assets = vector::empty<address>();
        let i = 0;
        while (i < N_ASSETS) {
            let symbol = b"DX";
            vector::push_back(&mut symbol, 48 + (i as u8));
            dexr_assets::create_asset_entry(admin, symbol, DECIMALS);
            vector::push_back(&mut assets, dexr_assets::asset_address(@bench, symbol));
            i = i + 1;
        };
        assets
    }

    /// The init sequence a transaction harness submits, one call per
    /// transaction and in this order.
    fun setup(admin: &signer): vector<address> {
        let assets = make_assets(admin);
        let a0 = *vector::borrow(&assets, 0);
        let a1 = *vector::borrow(&assets, 1);
        dexr_router::initialize(admin);
        dexr_pool_cpmm::create_pool(admin, a0, a1, RESERVE, FEE_BPS);
        dexr_pool_stable::create_pool(admin, a0, a1, RESERVE, STABLE_FEE_BPS, AMP);
        dexr_pool_clmm::create_pool(
            admin, a0, a1, RESERVE, FEE_BPS, TICKS, TICK_LIQUIDITY, 7
        );
        dexr_pool_book::create_pool(
            admin, a0, a1, RESERVE, FEE_BPS, DEPTH, LEVEL_SIZE, 11
        );
        dexr_pool_cpmm::seed_pools(
            admin, assets, POOLS_PER_BACKEND, RESERVE, FEE_BPS, 1
        );
        dexr_pool_stable::seed_pools(
            admin, assets, POOLS_PER_BACKEND, RESERVE, STABLE_FEE_BPS, AMP, 2
        );
        dexr_pool_clmm::seed_pools(
            admin, assets, POOLS_PER_BACKEND, RESERVE, FEE_BPS, TICKS, TICK_LIQUIDITY, 3
        );
        dexr_pool_book::seed_pools(
            admin, assets, POOLS_PER_BACKEND, RESERVE, FEE_BPS, DEPTH, LEVEL_SIZE, 4
        );
        dexr_router::map_markers(admin, assets);
        assets
    }

    /// Two assets and nothing else, for the tests that pin one backend's price
    /// function.
    fun two_assets(admin: &signer): (address, address) {
        dexr_assets::create_asset_entry(admin, b"XX", DECIMALS);
        dexr_assets::create_asset_entry(admin, b"YY", DECIMALS);
        (
            dexr_assets::asset_address(@bench, b"XX"),
            dexr_assets::asset_address(@bench, b"YY"),
        )
    }

    fun assert_reserves_non_zero(pool_id: u64) {
        let (x, y) = dexr_pool_cpmm::reserves(pool_id);
        assert!(x > 0 && y > 0, 0);
        let (x, y) = dexr_pool_stable::reserves(pool_id);
        assert!(x > 0 && y > 0, 0);
        let (x, y) = dexr_pool_clmm::reserves(pool_id);
        assert!(x > 0 && y > 0, 0);
        let (x, y) = dexr_pool_book::reserves(pool_id);
        assert!(x > 0 && y > 0, 0);
    }

    fun assert_onboarded(who: address, assets: &vector<address>) {
        let i = 0;
        while (i < vector::length(assets)) {
            let metadata = dexr_assets::metadata_at(*vector::borrow(assets, i));
            assert!(dexr_assets::primary_balance(who, metadata) > 0, 0);
            i = i + 1;
        };
    }

    #[test(admin = @bench, alice = @0xa11ce, bob = @0xb0b)]
    fun test_harness_onboard_then_mix(admin: &signer, alice: &signer, bob: &signer) {
        let assets = setup(admin);
        dexr_router::bench_onboard(alice, FUNDING);
        dexr_router::bench_onboard(bob, FUNDING);
        assert_onboarded(signer::address_of(alice), &assets);
        assert_onboarded(signer::address_of(bob), &assets);

        dexr_router::bench_route1<Cpmm, A0, A1, M0>(alice, 0, AMOUNT);
        dexr_router::bench_route2<Cpmm, Stable, A0, A1, A2, M0, M1, M2>(
            alice, 1, AMOUNT
        );
        dexr_router::bench_route3<
            Cpmm, Stable, Clmm,
            A0, A1, A2, A3,
            M0, M1, M2, M3, M4, M5, M6, M7, M8,
        >(alice, 2, AMOUNT);
        dexr_router::bench_route5<
            Cpmm, Stable, Clmm, Book, Cpmm,
            A0, A1, A2, A3, A4, A5,
            M0, M1, M2, M3, M4, M5, M6,
            M7, M8, M9, M10, M11, M12, M13,
            M14, M15, M16, M17, M18, M19, M20,
        >(bob, 3, AMOUNT);
        dexr_router::bench_split<
            Cpmm, Stable, Clmm, Book,
            A0, A1, A2,
            M0, M1, M2, M3, M4, M5, M6, M7, M8,
        >(bob, 4, AMOUNT, 5000);
        // The instantiation the mix emits when the asset slot and the mode
        // slot both land on four. They are drawn independently, so a start
        // partway into each marker list is an ordinary case rather than a
        // corner one.
        dexr_router::bench_quote<
            Cpmm, Stable, Clmm,
            A4, A5, A6, A7,
            M4, M5, M6, M7, M8, M9, M10, M11, M12,
        >(bob, 5, AMOUNT);
        dexr_router::bench_rebalance(bob, 0, 0, AMOUNT);

        assert_reserves_non_zero(0);
        assert_reserves_non_zero(1);
        assert_onboarded(signer::address_of(alice), &assets);
        assert_onboarded(signer::address_of(bob), &assets);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_route5_verifies_at_32_type_params(admin: &signer, alice: &signer) {
        setup(admin);
        dexr_router::bench_onboard(alice, FUNDING);
        dexr_router::bench_route5<
            Cpmm, Stable, Clmm, Book, Cpmm,
            A0, A1, A2, A3, A4, A5,
            M0, M1, M2, M3, M4, M5, M6,
            M7, M8, M9, M10, M11, M12, M13,
            M14, M15, M16, M17, M18, M19, M20,
        >(alice, 0, AMOUNT);
        assert_reserves_non_zero(0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_route_tolerates_unmapped_marker(admin: &signer, alice: &signer) {
        setup(admin);
        dexr_router::bench_onboard(alice, FUNDING);
        // Mode markers are never bound, so both legs fall back to asset zero.
        assert!(dexr_backend::resolve<M9>() == dexr_backend::resolve<A0>(), 0);
        dexr_router::bench_route2<Cpmm, Book, M9, M10, M11, M0, M1, M2>(
            alice, 0, AMOUNT
        );
        assert_reserves_non_zero(0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_route_tolerates_out_of_range_pool_id(admin: &signer, alice: &signer) {
        setup(admin);
        dexr_router::bench_onboard(alice, FUNDING);
        assert!(dexr_backend::n_pools(0) < 1_000_000, 0);
        dexr_router::bench_route1<Cpmm, A0, A1, M0>(alice, 1_000_000, AMOUNT);
        dexr_router::bench_rebalance(alice, 3, 1_000_000, AMOUNT);
        assert_reserves_non_zero(0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_route_tolerates_amount_past_reserve(admin: &signer, alice: &signer) {
        setup(admin);
        dexr_router::bench_onboard(alice, FUNDING);
        dexr_router::bench_route2<Stable, Cpmm, A0, A1, A2, M0, M1, M2>(
            alice, 0, U64_MAX
        );
        assert_reserves_non_zero(0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_clmm_tolerates_exhausted_ticks(admin: &signer, alice: &signer) {
        let (x, y) = two_assets(admin);
        dexr_pool_clmm::initialize(admin);
        let pool_id = dexr_pool_clmm::n_pools();
        // Two ticks of one unit each: any trade walks off the end.
        dexr_pool_clmm::create_pool(admin, x, y, RESERVE, FEE_BPS, 2, 1, 5);
        assert!(dexr_pool_clmm::live_ticks(pool_id) == 2, 0);
        let _ = dexr_pool_clmm::swap(alice, pool_id, x, y, AMOUNT);
        assert!(dexr_pool_clmm::live_ticks(pool_id) == 2, 0);
        let _ = dexr_pool_clmm::swap(alice, pool_id, x, y, AMOUNT);
        let (rx, ry) = dexr_pool_clmm::reserves(pool_id);
        assert!(rx > 0 && ry > 0, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_book_tolerates_exhausted_levels(admin: &signer, alice: &signer) {
        let (x, y) = two_assets(admin);
        dexr_pool_book::initialize(admin);
        let pool_id = dexr_pool_book::n_pools();
        // Two levels of one unit each: any trade clears the whole book.
        dexr_pool_book::create_pool(admin, x, y, RESERVE, FEE_BPS, 2, 1, 9);
        assert!(dexr_pool_book::live_levels(pool_id) == 2, 0);
        let _ = dexr_pool_book::swap(alice, pool_id, x, y, AMOUNT);
        assert!(dexr_pool_book::live_levels(pool_id) == 2, 0);
        let _ = dexr_pool_book::swap(alice, pool_id, x, y, AMOUNT);
        let (rx, ry) = dexr_pool_book::reserves(pool_id);
        assert!(rx > 0 && ry > 0, 0);
    }

    // A seed past `u64::MAX / LCG_MUL` is stored verbatim and replayed on
    // every wrap and every rebalance, so an unreduced state would abort a
    // route that lands on the pool.
    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_wide_seed_does_not_overflow(admin: &signer, alice: &signer) {
        let (x, y) = two_assets(admin);
        dexr_pool_clmm::initialize(admin);
        dexr_pool_book::initialize(admin);
        dexr_pool_cpmm::initialize(admin);

        // Two ticks of one unit each, so every trade walks off the end and
        // rebuilds the range from the stored seed.
        let clmm_id = dexr_pool_clmm::n_pools();
        dexr_pool_clmm::create_pool(admin, x, y, RESERVE, FEE_BPS, 2, 1, U64_MAX);
        let _ = dexr_pool_clmm::swap(alice, clmm_id, x, y, AMOUNT);
        dexr_pool_clmm::rebalance(clmm_id, AMOUNT);
        assert!(dexr_pool_clmm::live_ticks(clmm_id) == 2, 0);

        let book_id = dexr_pool_book::n_pools();
        dexr_pool_book::create_pool(admin, x, y, RESERVE, FEE_BPS, 2, 1, U64_MAX);
        let _ = dexr_pool_book::swap(alice, book_id, x, y, AMOUNT);
        dexr_pool_book::rebalance(book_id, AMOUNT);
        assert!(dexr_pool_book::live_levels(book_id) == 2, 0);

        let pair = vector::empty<address>();
        vector::push_back(&mut pair, x);
        vector::push_back(&mut pair, y);
        let before = dexr_pool_cpmm::n_pools();
        dexr_pool_cpmm::seed_pools(admin, pair, 2, RESERVE, FEE_BPS, U64_MAX);
        assert!(dexr_pool_cpmm::n_pools() == before + 2, 0);
    }

    // An accessor called before its backend was initialized reads as empty
    // rather than aborting.
    #[test]
    fun test_accessors_tolerate_missing_backend() {
        let (x, y) = dexr_pool_cpmm::reserves(0);
        assert!(x == 0 && y == 0, 0);
        let (x, y) = dexr_pool_stable::reserves(0);
        assert!(x == 0 && y == 0, 0);
        let (x, y) = dexr_pool_clmm::reserves(0);
        assert!(x == 0 && y == 0, 0);
        let (x, y) = dexr_pool_book::reserves(0);
        assert!(x == 0 && y == 0, 0);
        assert!(dexr_pool_clmm::live_ticks(0) == 0, 0);
        assert!(dexr_pool_book::live_levels(0) == 0, 0);
        let (liquidity, _, _) = dexr_pool_clmm::tick_at(0, 0);
        assert!(liquidity == 0, 0);
        let (size, _, _) = dexr_pool_book::level_at(0, 0);
        assert!(size == 0, 0);
    }

    // One `seed_pools` call at the batch size and pool shape the harness
    // deploys. The tick-bearing backends are the heavy ones: the same batch
    // on a constant product or stable backend does the same vault work with
    // no range vector.
    #[test(admin = @bench)]
    fun test_seed_batch_clmm(admin: &signer) {
        let assets = make_assets(admin);
        dexr_pool_clmm::initialize(admin);
        dexr_pool_clmm::seed_pools(
            admin,
            assets,
            SEED_BATCH,
            DEPLOYED_RESERVE,
            FEE_BPS,
            DEPLOYED_RANGE,
            DEPLOYED_UNIT,
            3,
        );
        assert!(dexr_pool_clmm::n_pools() == SEED_BATCH, 0);
        assert!(dexr_pool_clmm::live_ticks(0) == DEPLOYED_RANGE, 0);
        assert!(dexr_pool_clmm::live_ticks(SEED_BATCH - 1) == DEPLOYED_RANGE, 0);
    }

    #[test(admin = @bench)]
    fun test_seed_batch_book(admin: &signer) {
        let assets = make_assets(admin);
        dexr_pool_book::initialize(admin);
        dexr_pool_book::seed_pools(
            admin,
            assets,
            SEED_BATCH,
            DEPLOYED_RESERVE,
            FEE_BPS,
            DEPLOYED_RANGE,
            DEPLOYED_UNIT,
            4,
        );
        assert!(dexr_pool_book::n_pools() == SEED_BATCH, 0);
        assert!(dexr_pool_book::live_levels(0) == DEPLOYED_RANGE, 0);
        assert!(dexr_pool_book::live_levels(SEED_BATCH - 1) == DEPLOYED_RANGE, 0);
    }

    #[test]
    fun test_math_mul_div() {
        assert!(dexr_math::mul_div(7, 3, 2) == 10, 0);
        assert!(dexr_math::mul_div(5, 5, 0) == 0, 0);
        // The product overflows `u64` but the quotient does not.
        assert!(dexr_math::mul_div(U64_MAX, 4, 4) == U64_MAX, 0);
        // The quotient overflows too, and saturates instead of aborting.
        assert!(dexr_math::mul_div(U64_MAX, 4, 1) == U64_MAX, 0);
    }

    #[test]
    fun test_math_clamp_in() {
        assert!(dexr_math::clamp_in(1_000_000, 1_000_000) == 50_000, 0);
        assert!(dexr_math::clamp_in(100, 1_000) == 50, 0);
        assert!(dexr_math::clamp_in(100, 19) == 0, 0);
        assert!(dexr_math::clamp_in(100, 0) == 0, 0);
        // A reserve already at the ceiling accepts nothing.
        assert!(dexr_math::clamp_in(1, 1_000_000_000_000_000_000) == 0, 0);
    }

    #[test]
    fun test_math_headroom() {
        assert!(dexr_math::headroom(0) == 1_000_000_000_000_000_000, 0);
        assert!(dexr_math::headroom(1_000_000_000_000_000_000) == 0, 0);
        assert!(dexr_math::headroom(U64_MAX) == 0, 0);
    }

    #[test]
    fun test_math_stable_invariant() {
        // At the balanced point the invariant is exactly twice one side, and
        // the other side that holds it is the side you started with.
        let ann = dexr_math::amp_n_n(AMP);
        assert!(ann == 400, 0);
        assert!(dexr_math::invariant_d(1_000_000, 1_000_000, ann) == 2_000_000, 0);
        assert!(dexr_math::solve_y(1_000_000, 2_000_000, ann) == 1_000_000, 0);
        // A zero balance has no invariant to solve.
        assert!(dexr_math::invariant_d(0, 1_000_000, ann) == 0, 0);
        assert!(dexr_math::solve_y(0, 2_000_000, ann) == 0, 0);
    }

    #[test(admin = @bench)]
    fun test_cpmm_price(admin: &signer) {
        let (x, y) = two_assets(admin);
        dexr_pool_cpmm::initialize(admin);
        let pool_id = dexr_pool_cpmm::n_pools();
        dexr_pool_cpmm::create_pool(admin, x, y, 1_000_000, FEE_BPS);
        // 1000 in, 0.3% fee: 997 reaches the curve and 997e6/1000997 floors
        // to 996.
        assert!(dexr_math::cpmm_out(1000, 1_000_000, 1_000_000, FEE_BPS) == 996, 0);
        assert!(dexr_pool_cpmm::quote(pool_id, x, y, 1000) == 996, 0);
        // A fee at or above the denominator leaves one basis point through:
        // 100 reaches the curve and 100e6/1000100 floors to 99.
        assert!(dexr_math::cpmm_out(1_000_000, 1_000_000, 1_000_000, 10_000) == 99, 0);
    }

    #[test(admin = @bench)]
    fun test_stable_price(admin: &signer) {
        let (x, y) = two_assets(admin);
        dexr_pool_stable::initialize(admin);
        let pool_id = dexr_pool_stable::n_pools();
        dexr_pool_stable::create_pool(admin, x, y, 1_000_000, 0, AMP);
        let out = dexr_pool_stable::quote(pool_id, x, y, 50_000);
        // Amplification holds a balanced pool near one to one, which is the
        // whole point of the curve.
        assert!(out > 49_000 && out < 50_000, 0);
        assert!(out > dexr_math::cpmm_out(50_000, 1_000_000, 1_000_000, 0), 0);
    }

    #[test(admin = @bench)]
    fun test_clmm_price(admin: &signer) {
        let (x, y) = two_assets(admin);
        dexr_pool_clmm::initialize(admin);
        let pool_id = dexr_pool_clmm::n_pools();
        dexr_pool_clmm::create_pool(
            admin, x, y, 1_000_000_000, FEE_BPS, TICKS, TICK_LIQUIDITY, 7
        );
        let (liquidity, price_num, price_den) = dexr_pool_clmm::tick_at(pool_id, 0);
        assert!(liquidity > 0, 0);
        // A trade inside the first tick prices at that tick alone.
        let amount = liquidity / 2;
        assert!(
            dexr_pool_clmm::quote(pool_id, x, y, amount)
                == dexr_math::mul_div(amount, price_num, price_den),
            0,
        );
        // Ticks price strictly worse the further the walk goes.
        let (_, deep_num, deep_den) = dexr_pool_clmm::tick_at(pool_id, TICKS - 1);
        assert!(price_num <= price_den, 0);
        assert!(
            dexr_math::mul_div(1_000_000, deep_num, deep_den)
                < dexr_math::mul_div(1_000_000, price_num, price_den),
            0,
        );
    }

    #[test(admin = @bench)]
    fun test_book_price(admin: &signer) {
        let (x, y) = two_assets(admin);
        dexr_pool_book::initialize(admin);
        let pool_id = dexr_pool_book::n_pools();
        // One level, so the walk cannot leak into a second price.
        dexr_pool_book::create_pool(admin, x, y, 1_000_000_000, FEE_BPS, 1, LEVEL_SIZE, 11);
        let (size, price_num, price_den) = dexr_pool_book::level_at(pool_id, 0);
        assert!(size > 0, 0);
        // A trade that does not clear the top level fills at that level alone.
        let amount = size / 2;
        assert!(
            dexr_pool_book::quote(pool_id, x, y, amount)
                == dexr_math::mul_div(amount, price_num, price_den),
            0,
        );
        // Levels are priced below one, so a fill gives back less than it took.
        assert!(price_num <= price_den, 0);

        // Prices worsen with depth, so clearing more levels fills worse.
        let deep_id = dexr_pool_book::n_pools();
        dexr_pool_book::create_pool(
            admin, x, y, 1_000_000_000, FEE_BPS, DEPTH, LEVEL_SIZE, 11
        );
        let (_, top_num, top_den) = dexr_pool_book::level_at(deep_id, 0);
        let (_, deep_num, deep_den) = dexr_pool_book::level_at(deep_id, DEPTH - 1);
        assert!(
            dexr_math::mul_div(1_000_000, deep_num, deep_den)
                < dexr_math::mul_div(1_000_000, top_num, top_den),
            0,
        );
    }

    #[test(admin = @bench)]
    fun test_backend_dispatch(admin: &signer) {
        let assets = make_assets(admin);
        dexr_router::initialize(admin);
        dexr_router::map_markers(admin, assets);
        assert!(dexr_backend::tag<Cpmm>() == 0, 0);
        assert!(dexr_backend::tag<Stable>() == 1, 0);
        assert!(dexr_backend::tag<Clmm>() == 2, 0);
        assert!(dexr_backend::tag<Book>() == 3, 0);
        // An unknown venue marker takes the constant product path.
        assert!(dexr_backend::tag<M0>() == 0, 0);
        assert!(dexr_backend::resolve<A3>() == *vector::borrow(&assets, 3), 0);
        assert!(dexr_backend::mode_bits<M0>() < 4, 0);
        assert!(dexr_backend::mode_bits3<M0, M1, M2>() <= 9, 0);
        assert!(dexr_backend::mode_bits7<M0, M1, M2, M3, M4, M5, M6>() <= 21, 0);
    }
}
