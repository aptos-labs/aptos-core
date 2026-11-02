#[test_only]
module bench::lending_market_tests {
    use std::signer;
    use std::vector;
    use aptos_framework::timestamp;
    use bench::lending_config;
    use bench::lending_logic;
    use bench::lending_math;
    use bench::lending_assets;
    use bench::lending_pool;
    use bench::lending_tokens;

    const RAY: u256 = 1_000_000_000_000_000_000_000_000_000;
    const WAD: u256 = 1_000_000_000_000_000_000;
    const YEAR: u64 = 31_536_000;
    const MONTH: u64 = 2_592_000;

    /// Reserve timestamps are microseconds, so a formula called with an
    /// explicit timestamp needs this rather than `YEAR`.
    const YEAR_MICROS: u64 = 31_536_000_000_000;
    const VARIABLE: u8 = 2;

    /// Every argument the `lending_market` generator in `bench_workflows.rs`
    /// issues. `test_harness_onboard_then_mix` replays the generator, so these
    /// have to move together with it.
    const NUM_RESERVES: u64 = 8;
    const COLLATERALS_PER_ACCOUNT: u64 = 3;
    const RESERVE_FACTOR_BPS: u256 = 1000;
    const PROTOCOL_FEE_BPS: u256 = 0;
    const BOOTSTRAP_SUPPLY: u256 = 1_000_000_000_000;
    const ONBOARD_SUPPLY: u256 = 1_000_000_000;
    const ONBOARD_BORROW: u256 = 100_000_000;
    const STEP: u256 = 10_000;
    const FLASH_AMOUNT: u256 = 1_000_000;
    const FLASH_PREMIUM_BPS: u256 = 9;
    const FLASH_RECEIVER_OPS: u64 = 8;

    fun start(aptos_framework: &signer, admin: &signer) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        lending_pool::initialize(admin);
    }

    fun add_reserve(
        admin: &signer,
        symbol: vector<u8>,
        decimals: u8,
        price: u256,
        ltv: u256,
        threshold: u256,
        bonus: u256,
        reserve_factor: u256,
        protocol_fee: u256
    ): address {
        lending_assets::create_asset_entry(admin, symbol, decimals);
        let asset = lending_assets::asset_address(symbol);
        lending_pool::admin_add_reserve(
            admin,
            asset,
            ltv,
            threshold,
            bonus,
            reserve_factor,
            protocol_fee,
            8000,
            0,
            400,
            7500
        );
        lending_pool::oracle_set_price(admin, asset, price);
        asset
    }

    fun scaled_debt(user: address, asset: address): u256 {
        lending_tokens::scaled_balance_of(user, lending_pool::get_variable_debt_token(asset))
    }

    fun scaled_collateral(user: address, asset: address): u256 {
        lending_tokens::scaled_balance_of(user, lending_pool::get_a_token(asset))
    }

    #[test]
    fun test_fixed_point_rounding_is_half_up() {
        assert!(lending_math::ray_mul(RAY, RAY) == RAY, 1);
        assert!(lending_math::ray_div(RAY, RAY) == RAY, 2);
        assert!(lending_math::wad_mul(WAD, WAD) == WAD, 3);
        assert!(lending_math::wad_div(WAD, WAD) == WAD, 4);

        // Exactly one half rounds away from zero, one unit below it does not.
        assert!(lending_math::ray_mul(1, RAY / 2) == 1, 5);
        assert!(lending_math::ray_mul(1, RAY / 2 - 1) == 0, 6);
        assert!(lending_math::ray_div(1, 2 * RAY) == 1, 7);
        assert!(lending_math::ray_div(1, 2 * RAY + 2) == 0, 8);
        assert!(lending_math::wad_mul(1, WAD / 2) == 1, 9);
        assert!(lending_math::wad_mul(1, WAD / 2 - 1) == 0, 10);
        assert!(lending_math::wad_div(1, 2 * WAD) == 1, 11);
        assert!(lending_math::wad_div(1, 2 * WAD + 2) == 0, 12);

        assert!(lending_math::ray_mul_up(1, 1) == 1, 13);
        assert!(lending_math::ray_mul_down(1, 1) == 0, 14);
        assert!(lending_math::ray_div_up(1, 3 * RAY) == 1, 15);
        assert!(lending_math::ray_div_down(1, 3 * RAY) == 0, 16);

        assert!(lending_math::percent_mul(1, 5000) == 1, 17);
        assert!(lending_math::percent_mul(1, 4999) == 0, 18);
        assert!(lending_math::percent_div(1, 20000) == 1, 19);
        assert!(lending_math::percent_div(1, 20002) == 0, 20);

        assert!(lending_math::ceil_div(7, 3) == 3, 21);
        assert!(lending_math::ceil_div(9, 3) == 3, 22);
        assert!(lending_math::wad_to_ray(1) == 1_000_000_000, 23);
        assert!(lending_math::ray_to_wad(1_500_000_000) == 2, 24);
        assert!(lending_math::ray_to_wad(1_499_999_999) == 1, 25);
        assert!(lending_math::pow(10, 18) == WAD, 26);
        assert!(lending_math::bitwise_negation(0) == lending_math::u256_max(), 27);
    }

    #[test(aptos_framework = @0x1)]
    fun test_interest_formulas_match_known_ray_values(
        aptos_framework: &signer
    ) {
        timestamp::set_time_has_started_for_testing(aptos_framework);

        // Ten percent per year, half a year elapsed.
        timestamp::update_global_time_for_test_secs(YEAR / 2);
        assert!(
            lending_math::calculate_linear_interest(RAY / 10, 0)
                == 1_050_000_000_000_000_000_000_000_000,
            1
        );

        // 1 + x + x^2/2 + x^3/6 at x = 0.1 is 1.105166666..., against the true
        // e^0.1 = 1.10517091.
        assert!(
            lending_math::calculate_compounded_interest(RAY / 10, 0, YEAR_MICROS)
                == 1_105_166_666_666_666_666_666_666_667,
            2
        );
        // x = 0.025 gives 1.025315104166..., against e^0.025 = 1.02531512.
        assert!(
            lending_math::calculate_compounded_interest(RAY / 40, 0, YEAR_MICROS)
                == 1_025_315_104_166_666_666_666_666_667,
            3
        );
        assert!(lending_math::calculate_compounded_interest(RAY / 10, 0, 0) == RAY, 4);
    }

    #[test]
    fun test_reserve_config_round_trips_at_boundaries() {
        let config = lending_config::init_reserve_configuration();
        lending_config::set_ltv(&mut config, 65535);
        lending_config::set_liquidation_threshold(&mut config, 65535);
        lending_config::set_liquidation_bonus(&mut config, 65535);
        lending_config::set_decimals(&mut config, 18);
        lending_config::set_reserve_factor(&mut config, 65535);
        lending_config::set_borrow_cap(&mut config, 68719476735);
        lending_config::set_supply_cap(&mut config, 68719476735);
        lending_config::set_liquidation_protocol_fee(&mut config, 65535);
        lending_config::set_active(&mut config, true);
        lending_config::set_frozen(&mut config, true);
        lending_config::set_borrowing_enabled(&mut config, true);
        lending_config::set_paused(&mut config, true);
        lending_config::set_flashloan_enabled(&mut config, true);

        assert!(lending_config::get_ltv(&config) == 65535, 1);
        assert!(lending_config::get_liquidation_threshold(&config) == 65535, 2);
        assert!(lending_config::get_liquidation_bonus(&config) == 65535, 3);
        assert!(lending_config::get_decimals(&config) == 18, 4);
        assert!(lending_config::get_reserve_factor(&config) == 65535, 5);
        assert!(lending_config::get_borrow_cap(&config) == 68719476735, 6);
        assert!(lending_config::get_supply_cap(&config) == 68719476735, 7);
        assert!(lending_config::get_liquidation_protocol_fee(&config) == 65535, 8);
        let (active, frozen, borrowing, paused, flashloan) =
            lending_config::get_flags(&config);
        assert!(active && frozen && borrowing && paused && flashloan, 9);

        // Clearing one field must leave every neighbouring field untouched.
        lending_config::set_decimals(&mut config, 6);
        lending_config::set_frozen(&mut config, false);
        assert!(lending_config::get_decimals(&config) == 6, 10);
        assert!(!lending_config::get_frozen(&config), 11);
        assert!(lending_config::get_ltv(&config) == 65535, 12);
        assert!(lending_config::get_liquidation_threshold(&config) == 65535, 13);
        assert!(lending_config::get_liquidation_bonus(&config) == 65535, 14);
        assert!(lending_config::get_reserve_factor(&config) == 65535, 15);
        assert!(lending_config::get_borrow_cap(&config) == 68719476735, 16);
        assert!(lending_config::get_supply_cap(&config) == 68719476735, 17);
        assert!(lending_config::get_liquidation_protocol_fee(&config) == 65535, 18);
        assert!(lending_config::get_active(&config), 19);
        assert!(lending_config::get_borrowing_enabled(&config), 20);
        assert!(lending_config::get_paused(&config), 21);
        assert!(lending_config::get_flashloan_enabled(&config), 22);

        lending_config::set_ltv(&mut config, 0);
        lending_config::set_liquidation_threshold(&mut config, 0);
        lending_config::set_liquidation_bonus(&mut config, 0);
        lending_config::set_reserve_factor(&mut config, 0);
        lending_config::set_borrow_cap(&mut config, 0);
        lending_config::set_supply_cap(&mut config, 0);
        lending_config::set_liquidation_protocol_fee(&mut config, 0);
        lending_config::set_active(&mut config, false);
        lending_config::set_borrowing_enabled(&mut config, false);
        lending_config::set_paused(&mut config, false);
        lending_config::set_flashloan_enabled(&mut config, false);
        lending_config::set_decimals(&mut config, 6);
        assert!(
            lending_config::reserve_configuration_data(&config)
                == (6 << 48),
            23
        );

        let restored =
            lending_config::reserve_configuration_from_data(
                lending_config::reserve_configuration_data(&config)
            );
        assert!(lending_config::get_decimals(&restored) == 6, 24);
    }

    #[test]
    #[expected_failure(abort_code = 13, location = bench::lending_config)]
    fun test_reserve_config_rejects_decimals_below_minimum() {
        let config = lending_config::init_reserve_configuration();
        lending_config::set_decimals(&mut config, 5);
    }

    #[test]
    fun test_user_config_bits_at_both_ends_of_the_range() {
        let config = lending_config::init_user_configuration();
        assert!(lending_config::is_empty(&config), 1);

        lending_config::set_borrowing(&mut config, 0, true);
        assert!(lending_config::user_configuration_data(&config) == 1, 2);
        assert!(lending_config::is_borrowing(&config, 0), 3);
        assert!(!lending_config::is_using_as_collateral(&config, 0), 4);
        assert!(lending_config::is_using_as_collateral_or_borrowing(&config, 0), 5);
        assert!(lending_config::is_borrowing_any(&config), 6);
        assert!(!lending_config::is_using_as_collateral_any(&config), 7);

        lending_config::set_using_as_collateral(&mut config, 0, true);
        assert!(lending_config::user_configuration_data(&config) == 3, 8);
        assert!(lending_config::is_using_as_collateral(&config, 0), 9);

        // Index 127 is the last reserve two bits per reserve leaves room for.
        lending_config::set_borrowing(&mut config, 127, true);
        lending_config::set_using_as_collateral(&mut config, 127, true);
        assert!(lending_config::is_borrowing(&config, 127), 10);
        assert!(lending_config::is_using_as_collateral(&config, 127), 11);
        assert!(
            lending_config::user_configuration_data(&config)
                == 3 + (1u256 << 254) + (1u256 << 255),
            12
        );
        assert!(!lending_config::is_borrowing(&config, 126), 13);
        assert!(!lending_config::is_using_as_collateral(&config, 126), 14);

        lending_config::set_borrowing(&mut config, 0, false);
        assert!(!lending_config::is_borrowing(&config, 0), 15);
        assert!(lending_config::is_using_as_collateral(&config, 0), 16);
        assert!(lending_config::is_borrowing_one(&config), 17);

        lending_config::set_using_as_collateral(&mut config, 127, false);
        assert!(lending_config::is_using_as_collateral_one(&config), 18);
        assert!(!lending_config::is_using_as_collateral(&config, 127), 19);
        assert!(lending_config::is_borrowing(&config, 127), 20);

        lending_config::set_borrowing(&mut config, 127, false);
        lending_config::set_using_as_collateral(&mut config, 0, false);
        assert!(lending_config::is_empty(&config), 21);
    }

    #[test]
    #[expected_failure(abort_code = 18, location = bench::lending_config)]
    fun test_user_config_rejects_index_past_the_range() {
        let config = lending_config::init_user_configuration();
        lending_config::set_borrowing(&mut config, 128, true);
    }

    #[test(aptos_framework = @0x1, admin = @bench, user = @0x123)]
    fun test_index_accrual_against_known_ray_values(
        aptos_framework: &signer, admin: &signer, user: &signer
    ) {
        start(aptos_framework, admin);
        let asset = add_reserve(admin, b"ACCR", 6, 100_000_000, 8000, 8500, 10500, 0, 0);
        let user_addr = signer::address_of(user);
        lending_assets::mint(admin, asset, user_addr, 1_000_000);

        lending_logic::supply(user, asset, 1_000_000);
        lending_logic::set_user_use_reserve_as_collateral(user, asset, true);
        lending_logic::borrow(user, asset, 500_000, VARIABLE);

        // Half the liquidity is borrowed, so the two-slope model sits on the
        // first slope: 4% * 0.5 / 0.8 = 2.5% borrow, 2.5% * 0.5 = 1.25% supply.
        assert!(
            lending_pool::get_current_variable_borrow_rate(asset)
                == 25_000_000_000_000_000_000_000_000,
            1
        );
        assert!(
            lending_pool::get_current_liquidity_rate(asset)
                == 12_500_000_000_000_000_000_000_000,
            2
        );

        timestamp::fast_forward_seconds(YEAR);

        assert!(
            lending_pool::get_normalized_income(asset)
                == 1_012_500_000_000_000_000_000_000_000,
            3
        );
        assert!(
            lending_pool::get_normalized_debt(asset)
                == 1_025_315_104_166_666_666_666_666_667,
            4
        );

        let cache = lending_pool::cache(asset);
        lending_pool::update_state(&mut cache);
        assert!(
            lending_pool::get_liquidity_index(asset)
                == 1_012_500_000_000_000_000_000_000_000,
            5
        );
        assert!(
            lending_pool::get_variable_borrow_index(asset)
                == 1_025_315_104_166_666_666_666_666_667,
            6
        );

        // A scaled balance is untouched by accrual; only the index moved.
        assert!(scaled_collateral(user_addr, asset) == 1_000_000, 7);
        assert!(
            lending_tokens::balance_of(
                user_addr,
                lending_pool::get_a_token(asset),
                lending_pool::get_liquidity_index(asset)
            ) == 1_012_500,
            8
        );
    }

    #[test(aptos_framework = @0x1, admin = @bench, user = @0x123)]
    fun test_health_factor_for_a_two_asset_user(
        aptos_framework: &signer, admin: &signer, user: &signer
    ) {
        start(aptos_framework, admin);
        // $1000 of collateral at 85% and $1000 at 60% carry $400 of debt.
        let a = add_reserve(admin, b"HFA", 6, 100_000_000, 8000, 8500, 10500, 0, 0);
        let b = add_reserve(admin, b"HFB", 6, 200_000_000, 5000, 6000, 10500, 0, 0);
        let user_addr = signer::address_of(user);
        lending_assets::mint(admin, a, user_addr, 1_000_000_000);
        lending_assets::mint(admin, b, user_addr, 500_000_000);

        lending_logic::supply(user, a, 1_000_000_000);
        lending_logic::supply(user, b, 500_000_000);
        lending_logic::set_user_use_reserve_as_collateral(user, a, true);
        lending_logic::set_user_use_reserve_as_collateral(user, b, true);
        lending_logic::borrow(user, a, 400_000_000, VARIABLE);

        let (collateral, debt, ltv, threshold, hf, zero_ltv) =
            lending_logic::user_account_data(user_addr);
        assert!(collateral == 200_000_000_000, 1);
        assert!(debt == 40_000_000_000, 2);
        assert!(ltv == 6500, 3);
        assert!(threshold == 7250, 4);
        // 2000 * 0.725 / 400 = 3.625.
        assert!(hf == 3_625_000_000_000_000_000, 5);
        assert!(!zero_ltv, 6);

        assert!(
            lending_logic::calculate_available_borrows(collateral, debt, ltv)
                == 90_000_000_000,
            7
        );
    }

    #[test(aptos_framework = @0x1, admin = @bench, user = @0x123)]
    #[expected_failure(abort_code = 71, location = bench::lending_logic)]
    fun test_borrow_past_the_ltv_aborts(
        aptos_framework: &signer, admin: &signer, user: &signer
    ) {
        start(aptos_framework, admin);
        let a = add_reserve(admin, b"LTVA", 6, 100_000_000, 8000, 8500, 10500, 0, 0);
        let user_addr = signer::address_of(user);
        lending_assets::mint(admin, a, user_addr, 1_000_000_000);

        lending_logic::supply(user, a, 1_000_000_000);
        lending_logic::set_user_use_reserve_as_collateral(user, a, true);
        // $1000 of collateral at 80% supports $800, not $900.
        lending_logic::borrow(user, a, 900_000_000, VARIABLE);
    }

    #[test(
        aptos_framework = @0x1, admin = @bench, user = @0x123, provider = @0x456
    )]
    #[expected_failure(abort_code = 70, location = bench::lending_logic)]
    fun test_borrow_below_health_factor_one_aborts(
        aptos_framework: &signer,
        admin: &signer,
        user: &signer,
        provider: &signer
    ) {
        start(aptos_framework, admin);
        let (collateral_asset, debt_asset, user_addr, _) =
            underwater_fixture(admin, user, provider);
        assert!(lending_logic::health_factor(user_addr) < WAD, 1);
        assert!(collateral_asset != debt_asset, 2);
        lending_logic::borrow(user, debt_asset, 1, VARIABLE);
    }

    /// A user holding $1000 of collateral against $700 of debt in another
    /// asset, then pushed under a health factor of one by a 30% price drop.
    fun underwater_fixture(
        admin: &signer, user: &signer, provider: &signer
    ): (address, address, address, address) {
        let collateral_asset =
            add_reserve(admin, b"LQCOL", 6, 100_000_000, 8000, 8500, 10500, 0, 0);
        let debt_asset =
            add_reserve(admin, b"LQDBT", 6, 100_000_000, 8000, 8500, 10500, 0, 0);
        let user_addr = signer::address_of(user);
        let provider_addr = signer::address_of(provider);

        lending_assets::mint(admin, collateral_asset, user_addr, 1_000_000_000);
        lending_assets::mint(admin, debt_asset, provider_addr, 1_000_000_000);

        lending_logic::supply(provider, debt_asset, 1_000_000_000);
        lending_logic::supply(user, collateral_asset, 1_000_000_000);
        lending_logic::set_user_use_reserve_as_collateral(user, collateral_asset, true);
        lending_logic::borrow(user, debt_asset, 700_000_000, VARIABLE);
        assert!(
            lending_logic::health_factor(user_addr) == 1_214_285_714_285_714_286, 100
        );

        lending_pool::oracle_set_price(admin, collateral_asset, 70_000_000);
        assert!(lending_logic::health_factor(user_addr) == 850_000_000_000_000_000, 101);
        (collateral_asset, debt_asset, user_addr, provider_addr)
    }

    #[test(
        aptos_framework = @0x1,
        admin = @bench,
        user = @0x123,
        provider = @0x456,
        liquidator = @0x789
    )]
    fun test_liquidation_takes_collateral_with_the_bonus(
        aptos_framework: &signer,
        admin: &signer,
        user: &signer,
        provider: &signer,
        liquidator: &signer
    ) {
        start(aptos_framework, admin);
        let (collateral_asset, debt_asset, user_addr, _) =
            underwater_fixture(admin, user, provider);
        let liquidator_addr = signer::address_of(liquidator);
        lending_assets::mint(admin, debt_asset, liquidator_addr, 100_000_000);

        lending_logic::liquidation_call(
            liquidator,
            collateral_asset,
            debt_asset,
            user_addr,
            100_000_000,
            false
        );

        // $100 of debt at $0.70 a unit buys 142.857142 units, and the 105%
        // bonus lifts that to 149.999999.
        assert!(
            lending_assets::balance_of(liquidator_addr, collateral_asset) == 149_999_999,
            1
        );
        assert!(lending_assets::balance_of(liquidator_addr, debt_asset) == 0, 2);
        assert!(scaled_collateral(user_addr, collateral_asset) == 850_000_001, 3);
        assert!(scaled_debt(user_addr, debt_asset) == 600_000_000, 4);
        // Below the break-even health factor of one over the bonus, closing
        // debt at a premium leaves the position slightly worse off.
        assert!(lending_logic::health_factor(user_addr) == 842_916_667_666_666_667, 5);

        let config = lending_pool::get_user_config(user_addr);
        assert!(
            lending_config::is_using_as_collateral(
                &config, (lending_pool::get_reserve_id(collateral_asset) as u256)
            ),
            6
        );
        assert!(
            lending_config::is_borrowing(
                &config, (lending_pool::get_reserve_id(debt_asset) as u256)
            ),
            7
        );
    }

    #[test(
        aptos_framework = @0x1, admin = @bench, provider = @0x456, borrower = @0x789
    )]
    fun test_flash_loan_premium_reaches_suppliers(
        aptos_framework: &signer,
        admin: &signer,
        provider: &signer,
        borrower: &signer
    ) {
        start(aptos_framework, admin);
        let asset = add_reserve(admin, b"FLOK", 6, 100_000_000, 8000, 8500, 10500, 0, 0);
        let provider_addr = signer::address_of(provider);
        let borrower_addr = signer::address_of(borrower);
        lending_assets::mint(admin, asset, provider_addr, 1_000_000_000);
        lending_assets::mint(admin, asset, borrower_addr, 1_000_000);
        lending_logic::supply(provider, asset, 1_000_000_000);

        lending_logic::flash_loan_simple(borrower, asset, 100_000_000, 9, 64);

        // A 9 bps premium on 100 units is 0.09, paid out of the borrower's own
        // funds and folded into the index rather than minted.
        assert!(lending_assets::balance_of(borrower_addr, asset) == 1_000_000 - 90_000, 1);
        assert!(lending_pool::get_liquidity_index(asset) > RAY, 2);
        assert!(
            lending_tokens::balance_of(
                provider_addr,
                lending_pool::get_a_token(asset),
                lending_pool::get_liquidity_index(asset)
            ) == 1_000_090_000,
            3
        );
        assert!(lending_pool::vault_balance(asset) == 1_000_090_000, 4);
    }

    #[test(
        aptos_framework = @0x1, admin = @bench, provider = @0x456, borrower = @0x789
    )]
    #[expected_failure(abort_code = 65540, location = aptos_framework::fungible_asset)]
    fun test_flash_loan_not_repaid_aborts(
        aptos_framework: &signer,
        admin: &signer,
        provider: &signer,
        borrower: &signer
    ) {
        start(aptos_framework, admin);
        let asset = add_reserve(admin, b"FLNO", 6, 100_000_000, 8000, 8500, 10500, 0, 0);
        lending_assets::mint(admin, asset, signer::address_of(provider), 1_000_000_000);
        lending_logic::supply(provider, asset, 1_000_000_000);

        // The borrower holds nothing beyond the loan itself, so the premium
        // cannot be paid.
        let receipt =
            lending_logic::flash_loan_simple_take(borrower, asset, 100_000_000, 9);
        lending_logic::flash_loan_simple_repay(borrower, receipt);
    }

    #[test(
        aptos_framework = @0x1,
        admin = @bench,
        user1 = @0x123,
        user2 = @0x456
    )]
    fun test_end_to_end_four_reserves_two_users(
        aptos_framework: &signer,
        admin: &signer,
        user1: &signer,
        user2: &signer
    ) {
        start(aptos_framework, admin);

        let r0 = add_reserve(admin, b"E2E0", 6, 100_000_000, 8000, 8500, 10500, 1000, 1000);
        let r1 = add_reserve(admin, b"E2E1", 6, 100_000_000, 8000, 8500, 10500, 1000, 1000);
        let r2 =
            add_reserve(
                admin, b"E2E2", 8, 200_000_000_000, 8000, 8500, 10500, 1000, 1000
            );
        let r3 =
            add_reserve(
                admin, b"E2E3", 8, 3_000_000_000_000, 8000, 8500, 10500, 1000, 1000
            );
        assert!(lending_pool::reserves_count() == 4, 1);
        assert!(lending_pool::reserve_address_by_id(0) == r0, 2);
        assert!(lending_pool::reserve_address_by_id(3) == r3, 3);

        let addr1 = signer::address_of(user1);
        let addr2 = signer::address_of(user2);
        lending_assets::mint(admin, r0, addr1, 10_000_000_000);
        lending_assets::mint(admin, r1, addr1, 10_000_000_000);
        lending_assets::mint(admin, r2, addr1, 1_000_000_000);
        lending_assets::mint(admin, r2, addr2, 100_000_000);
        lending_assets::mint(admin, r3, addr2, 10_000_000);

        // user1 posts $5000 and $1000; user2 posts $2000 and $3000.
        lending_logic::supply(user1, r0, 5_000_000_000);
        lending_logic::supply(user1, r1, 1_000_000_000);
        lending_logic::set_user_use_reserve_as_collateral(user1, r0, true);
        lending_logic::set_user_use_reserve_as_collateral(user1, r1, true);

        lending_logic::supply(user2, r2, 100_000_000);
        lending_logic::supply(user2, r3, 10_000_000);
        lending_logic::set_user_use_reserve_as_collateral(user2, r2, true);
        lending_logic::set_user_use_reserve_as_collateral(user2, r3, true);

        lending_logic::borrow(user2, r0, 2_000_000_000, VARIABLE);
        lending_logic::borrow(user1, r2, 10_000_000, VARIABLE);

        let (collateral2, debt2, _, _, hf2, _) = lending_logic::user_account_data(addr2);
        assert!(collateral2 == 500_000_000_000, 4);
        assert!(debt2 == 200_000_000_000, 5);
        assert!(hf2 == 2_125_000_000_000_000_000, 6);

        // Without elapsed time every index stays at one ray and the accrual
        // paths are never exercised.
        timestamp::fast_forward_seconds(MONTH);
        assert!(lending_pool::get_liquidity_index(r0) == RAY, 7);
        assert!(lending_pool::get_normalized_income(r0) > RAY, 8);
        assert!(lending_pool::get_normalized_debt(r0) > RAY, 9);

        lending_logic::supply(user1, r0, 1_000_000_000);
        assert!(lending_pool::get_liquidity_index(r0) > RAY, 10);
        assert!(lending_pool::get_variable_borrow_index(r0) > RAY, 11);
        assert!(lending_pool::get_accrued_to_treasury(r0) > 0, 12);

        lending_logic::withdraw(user1, r1, 500_000_000, addr1);
        lending_logic::borrow(user1, r2, 5_000_000, VARIABLE);
        lending_logic::repay(user1, r2, 5_000_000, VARIABLE);

        lending_logic::set_user_use_reserve_as_collateral(user1, r1, false);
        let config1 = lending_pool::get_user_config(addr1);
        assert!(
            !lending_config::is_using_as_collateral(
                &config1, (lending_pool::get_reserve_id(r1) as u256)
            ),
            13
        );
        lending_logic::set_user_use_reserve_as_collateral(user1, r1, true);

        lending_logic::flash_loan_simple(user1, r0, 1_000_000_000, 9, 128);

        // A 90% drop on user2's larger collateral takes them under water.
        lending_pool::oracle_set_price(admin, r3, 100_000_000_000);
        assert!(lending_logic::health_factor(addr2) < WAD, 14);

        let debt_before = scaled_debt(addr2, r0);
        let collateral_before = scaled_collateral(addr2, r2);
        let held_before = lending_assets::balance_of(addr1, r2);
        lending_logic::liquidation_call(user1, r2, r0, addr2, 100_000_000, false);

        assert!(scaled_debt(addr2, r0) < debt_before, 15);
        assert!(scaled_collateral(addr2, r2) < collateral_before, 16);
        // $100 of debt buys $105 of collateral at $2000 a unit, less the 10%
        // protocol cut of the $5 bonus.
        assert!(lending_assets::balance_of(addr1, r2) - held_before == 5_225_000, 17);
        assert!(
            lending_tokens::scaled_balance_of(@bench, lending_pool::get_a_token(r2)) > 0,
            18
        );
        assert!(lending_logic::health_factor(addr2) < WAD, 19);
    }

    #[test(aptos_framework = @0x1, admin = @bench, provider = @0x300)]
    fun test_seed_users_lands_on_the_expected_health_factor(
        aptos_framework: &signer, admin: &signer, provider: &signer
    ) {
        start(aptos_framework, admin);

        // Uniform reserves, so every user ends on the same health factor no
        // matter which window the generator picked for them.
        let assets = vector[
            add_reserve(admin, b"SD00", 6, 100_000_000, 8000, 8500, 10500, 0, 0),
            add_reserve(admin, b"SD01", 6, 100_000_000, 8000, 8500, 10500, 0, 0),
            add_reserve(admin, b"SD02", 6, 100_000_000, 8000, 8500, 10500, 0, 0),
            add_reserve(admin, b"SD03", 6, 100_000_000, 8000, 8500, 10500, 0, 0)
        ];

        // Five users cannot cover four reserves, so fund every one of them
        // rather than relying on where the generator lands.
        let provider_addr = signer::address_of(provider);
        let r = 0;
        while (r < 4) {
            let asset = *vector::borrow(&assets, r);
            lending_assets::mint(admin, asset, provider_addr, 10_000_000_000);
            lending_logic::supply(provider, asset, 10_000_000_000);
            r = r + 1;
        };

        assert!(!lending_pool::seeded_user_exists(42, 0), 1);
        lending_logic::seed_users(admin, 5, 2, 1_000_000_000, 500_000_000, 42);

        let i = 0;
        while (i < 5) {
            let user = lending_pool::seeded_user_address(42, i);
            assert!(lending_pool::seeded_user_exists(42, i), 2);

            // Two collaterals of $1000 at a 0.85 threshold carrying $500 of
            // debt: 0.85 * 2000 / 500 = 3.4.
            assert!(lending_logic::health_factor(user) == 3_400_000_000_000_000_000, 3);

            let collateral = 0;
            let debt = 0;
            let r = 0;
            while (r < 4) {
                let asset = *vector::borrow(&assets, r);
                collateral = collateral + scaled_collateral(user, asset);
                debt = debt + scaled_debt(user, asset);
                r = r + 1;
            };
            assert!(collateral == 2_000_000_000, 4);
            assert!(debt == 500_000_000, 5);
            i = i + 1;
        };
    }

    // Fewer reserves than collateral slots leaves nothing to borrow against.
    #[test(aptos_framework = @0x1, admin = @bench)]
    #[expected_failure(abort_code = 79, location = bench::lending_logic)]
    fun test_seed_users_needs_a_reserve_to_borrow_from(
        aptos_framework: &signer, admin: &signer
    ) {
        start(aptos_framework, admin);
        add_reserve(admin, b"SD10", 6, 100_000_000, 8000, 8500, 10500, 0, 0);
        add_reserve(admin, b"SD11", 6, 100_000_000, 8000, 8500, 10500, 0, 0);
        lending_logic::seed_users(admin, 1, 2, 1_000_000_000, 500_000_000, 7);
    }

    // Every abort the mix could reach by raising an amount, driven past its
    // bound through the `bench_*` wrappers.
    #[test(aptos_framework = @0x1, admin = @bench, user = @0xbe2)]
    fun test_bench_wrappers_clamp_instead_of_aborting(
        aptos_framework: &signer, admin: &signer, user: &signer
    ) {
        start(aptos_framework, admin);
        let collateral =
            add_reserve(admin, b"CL0", 6, 100_000_000, 8000, 8500, 10500, 1000, 0);
        let debt = add_reserve(admin, b"CL1", 6, 100_000_000, 8000, 8500, 10500, 1000, 0);
        lending_logic::bench_supply(admin, collateral, BOOTSTRAP_SUPPLY);
        lending_logic::bench_supply(admin, debt, BOOTSTRAP_SUPPLY);

        let admin_addr = signer::address_of(admin);
        let user_addr = signer::address_of(user);
        lending_logic::bench_onboard(user, 0, 1, ONBOARD_SUPPLY, ONBOARD_BORROW);

        // No debt of this asset, and nothing supplied of that one.
        let supplied = scaled_collateral(user_addr, collateral);
        lending_logic::bench_repay(user, collateral, STEP, VARIABLE);
        lending_logic::bench_withdraw(user, debt, lending_math::u256_max(), user_addr);
        assert!(scaled_collateral(user_addr, collateral) == supplied, 1);
        assert!(scaled_collateral(user_addr, debt) == 0, 2);

        // Far past the LTV allowance, so the borrow lands exactly on it.
        lending_logic::bench_borrow(user, debt, BOOTSTRAP_SUPPLY, VARIABLE);
        assert!(lending_logic::health_factor(user_addr) >= WAD, 3);
        let borrowed = scaled_debt(user_addr, debt);
        assert!(borrowed > ONBOARD_BORROW, 4);
        // With the allowance spent, a second one has nothing left to draw.
        lending_logic::bench_borrow(user, debt, BOOTSTRAP_SUPPLY, VARIABLE);
        assert!(scaled_debt(user_addr, debt) == borrowed, 5);

        // Far past the reserve's liquidity, so the loan takes all of it.
        lending_logic::bench_flash_loan(
            user,
            collateral,
            BOOTSTRAP_SUPPLY * 1000,
            FLASH_PREMIUM_BPS,
            FLASH_RECEIVER_OPS
        );
        assert!(lending_pool::get_liquidity_index(collateral) > RAY, 6);

        // The admin holds more of the debt reserve than it still has liquid,
        // so the withdrawal clamps to the virtual balance and drains it.
        lending_logic::bench_withdraw(
            admin, debt, lending_math::u256_max(), admin_addr
        );
        assert!(lending_pool::get_virtual_underlying_balance(debt) == 0, 7);
        assert!(scaled_collateral(admin_addr, debt) > 0, 8);
        // Drained, so every further withdrawal is a no-op rather than an
        // underflow.
        lending_logic::bench_withdraw(admin, debt, STEP, admin_addr);
        lending_logic::bench_flash_loan(
            admin, debt, STEP, FLASH_PREMIUM_BPS, FLASH_RECEIVER_OPS
        );

        // A premium with no supply to spread it over leaves the index alone.
        let cache = lending_pool::cache(collateral);
        let index = lending_pool::cache_next_liquidity_index(&cache);
        assert!(
            lending_pool::cumulate_to_liquidity_index(&mut cache, 0, STEP) == index, 9
        );
    }

    /// The reserves the generator's `initialize_package` configures: its
    /// `SYMBOLS` and `PARAMS`, its shared rate strategy, and `BOOTSTRAP_SUPPLY`
    /// on every one of them.
    fun bench_reserves(admin: &signer): vector<address> {
        let symbols = vector[b"BA0", b"BA1", b"BA2", b"BA3", b"BA4", b"BA5", b"BA6",
            b"BA7"];
        // (ltv, liquidation threshold, liquidation bonus, price).
        let params: vector<vector<u256>> = vector[
            vector[8000, 8500, 10500, 100_000_000],
            vector[7500, 8000, 11000, 200_000_000],
            vector[7000, 7500, 11000, 50_000_000],
            vector[8250, 8600, 10400, 150_000_000],
            vector[6500, 7000, 11500, 300_000_000],
            vector[7700, 8200, 10800, 80_000_000],
            vector[8100, 8400, 10600, 120_000_000],
            vector[6000, 6500, 12000, 40_000_000]
        ];

        let assets = vector[];
        let i = 0;
        while (i < NUM_RESERVES) {
            let p = vector::borrow(&params, i);
            let asset =
                add_reserve(
                    admin,
                    *vector::borrow(&symbols, i),
                    6,
                    *vector::borrow(p, 3),
                    *vector::borrow(p, 0),
                    *vector::borrow(p, 1),
                    *vector::borrow(p, 2),
                    RESERVE_FACTOR_BPS,
                    PROTOCOL_FEE_BPS
                );
            lending_logic::bench_supply(admin, asset, BOOTSTRAP_SUPPLY);
            vector::push_back(&mut assets, asset);
            i = i + 1;
        };
        assets
    }

    /// One benchmark account's whole transaction stream: the onboarding
    /// transaction, then one call of every entry function the mix samples, with
    /// the arguments the generator builds for an account whose slot is `start`.
    fun run_bench_account(
        user: &signer, assets: &vector<address>, start: u64
    ) {
        lending_logic::bench_onboard(
            user,
            start,
            COLLATERALS_PER_ACCOUNT,
            ONBOARD_SUPPLY,
            ONBOARD_BORROW
        );

        let user_addr = signer::address_of(user);
        let debt =
            *vector::borrow(assets, (start + COLLATERALS_PER_ACCOUNT) % NUM_RESERVES);
        assert!(scaled_debt(user_addr, debt) > 0, 1);

        // Supply and withdraw sample one of the account's collaterals at
        // random, so every slot has to hold up.
        let k = 0;
        while (k < COLLATERALS_PER_ACCOUNT) {
            let collateral = *vector::borrow(assets, (start + k) % NUM_RESERVES);
            assert!(scaled_collateral(user_addr, collateral) > 0, 2);
            lending_logic::bench_supply(user, collateral, STEP);
            lending_logic::bench_withdraw(user, collateral, STEP, user_addr);
            k = k + 1;
        };

        lending_logic::bench_borrow(user, debt, STEP, VARIABLE);
        lending_logic::bench_repay(user, debt, STEP, VARIABLE);

        // The toggle always targets the last collateral, and the generator
        // alternates its flag, so both halves run. Clearing the bit is what
        // puts `validate_hf_and_ltv` on the measured path.
        let toggled =
            *vector::borrow(
                assets, (start + COLLATERALS_PER_ACCOUNT - 1) % NUM_RESERVES
            );
        let toggled_id = (lending_pool::get_reserve_id(toggled) as u256);
        lending_logic::set_user_use_reserve_as_collateral(user, toggled, false);
        assert!(
            !lending_config::is_using_as_collateral(
                &lending_pool::get_user_config(user_addr), toggled_id
            ),
            3
        );
        // Two collaterals still carry the one debt position, with the health
        // factor far above the liquidation threshold.
        assert!(lending_logic::health_factor(user_addr) > 6 * WAD, 4);
        lending_logic::set_user_use_reserve_as_collateral(user, toggled, true);
        assert!(
            lending_config::is_using_as_collateral(
                &lending_pool::get_user_config(user_addr), toggled_id
            ),
            5
        );

        let collateral = *vector::borrow(assets, start);
        lending_logic::bench_flash_loan(
            user,
            collateral,
            FLASH_AMOUNT,
            FLASH_PREMIUM_BPS,
            FLASH_RECEIVER_OPS
        );

        // The mix is net-neutral, so the position it started from survives it.
        assert!(scaled_debt(user_addr, debt) > 0, 6);
        assert!(scaled_collateral(user_addr, collateral) > 0, 7);
    }

    // The recipe the transaction generator follows: configure the reserves and
    // bootstrap their liquidity once as admin, onboard each account with one
    // transaction, then run the mix. Every step after onboarding must succeed
    // from a plain signer. Two accounts, on overlapping reserve windows, so one
    // reserve is collateral for both and another is one account's debt and the
    // other's collateral.
    #[test(aptos_framework = @0x1, admin = @bench, user1 = @0xbe0, user2 = @0xbe1)]
    fun test_harness_onboard_then_mix(
        aptos_framework: &signer,
        admin: &signer,
        user1: &signer,
        user2: &signer
    ) {
        start(aptos_framework, admin);
        let assets = bench_reserves(admin);
        assert!(lending_pool::reserves_count() == NUM_RESERVES, 1);
        run_bench_account(user1, &assets, 1);
        run_bench_account(user2, &assets, 3);
    }
}
