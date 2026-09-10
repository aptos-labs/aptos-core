#[test_only]
module bench::aave_tests {
    use std::signer;
    use std::vector;
    use aptos_framework::timestamp;
    use bench::aave_config;
    use bench::aave_logic;
    use bench::aave_math;
    use bench::aave_mock_fa;
    use bench::aave_pool;
    use bench::aave_tokens;

    const RAY: u256 = 1_000_000_000_000_000_000_000_000_000;
    const WAD: u256 = 1_000_000_000_000_000_000;
    const YEAR: u64 = 31_536_000;
    const MONTH: u64 = 2_592_000;

    /// Reserve timestamps are microseconds, so a formula called with an
    /// explicit timestamp needs this rather than `YEAR`.
    const YEAR_MICROS: u64 = 31_536_000_000_000;
    const VARIABLE: u8 = 2;

    fun start(aptos_framework: &signer, admin: &signer) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        aave_pool::initialize(admin);
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
        aave_mock_fa::create_asset(admin, symbol, decimals);
        let asset = aave_mock_fa::asset_address(symbol);
        aave_pool::admin_add_reserve(
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
        aave_pool::oracle_set_price(admin, asset, price);
        asset
    }

    fun scaled_debt(user: address, asset: address): u256 {
        aave_tokens::scaled_balance_of(user, aave_pool::get_variable_debt_token(asset))
    }

    fun scaled_collateral(user: address, asset: address): u256 {
        aave_tokens::scaled_balance_of(user, aave_pool::get_a_token(asset))
    }

    #[test]
    fun test_fixed_point_rounding_is_half_up() {
        assert!(aave_math::ray_mul(RAY, RAY) == RAY, 1);
        assert!(aave_math::ray_div(RAY, RAY) == RAY, 2);
        assert!(aave_math::wad_mul(WAD, WAD) == WAD, 3);
        assert!(aave_math::wad_div(WAD, WAD) == WAD, 4);

        // Exactly one half rounds away from zero, one unit below it does not.
        assert!(aave_math::ray_mul(1, RAY / 2) == 1, 5);
        assert!(aave_math::ray_mul(1, RAY / 2 - 1) == 0, 6);
        assert!(aave_math::ray_div(1, 2 * RAY) == 1, 7);
        assert!(aave_math::ray_div(1, 2 * RAY + 2) == 0, 8);
        assert!(aave_math::wad_mul(1, WAD / 2) == 1, 9);
        assert!(aave_math::wad_mul(1, WAD / 2 - 1) == 0, 10);
        assert!(aave_math::wad_div(1, 2 * WAD) == 1, 11);
        assert!(aave_math::wad_div(1, 2 * WAD + 2) == 0, 12);

        assert!(aave_math::ray_mul_up(1, 1) == 1, 13);
        assert!(aave_math::ray_mul_down(1, 1) == 0, 14);
        assert!(aave_math::ray_div_up(1, 3 * RAY) == 1, 15);
        assert!(aave_math::ray_div_down(1, 3 * RAY) == 0, 16);

        assert!(aave_math::percent_mul(1, 5000) == 1, 17);
        assert!(aave_math::percent_mul(1, 4999) == 0, 18);
        assert!(aave_math::percent_div(1, 20000) == 1, 19);
        assert!(aave_math::percent_div(1, 20002) == 0, 20);

        assert!(aave_math::ceil_div(7, 3) == 3, 21);
        assert!(aave_math::ceil_div(9, 3) == 3, 22);
        assert!(aave_math::wad_to_ray(1) == 1_000_000_000, 23);
        assert!(aave_math::ray_to_wad(1_500_000_000) == 2, 24);
        assert!(aave_math::ray_to_wad(1_499_999_999) == 1, 25);
        assert!(aave_math::pow(10, 18) == WAD, 26);
        assert!(aave_math::bitwise_negation(0) == aave_math::u256_max(), 27);
    }

    #[test(aptos_framework = @0x1)]
    fun test_interest_formulas_match_known_ray_values(
        aptos_framework: &signer
    ) {
        timestamp::set_time_has_started_for_testing(aptos_framework);

        // Ten percent per year, half a year elapsed.
        timestamp::update_global_time_for_test_secs(YEAR / 2);
        assert!(
            aave_math::calculate_linear_interest(RAY / 10, 0)
                == 1_050_000_000_000_000_000_000_000_000,
            1
        );

        // 1 + x + x^2/2 + x^3/6 at x = 0.1 is 1.105166666..., against the true
        // e^0.1 = 1.10517091.
        assert!(
            aave_math::calculate_compounded_interest(RAY / 10, 0, YEAR_MICROS)
                == 1_105_166_666_666_666_666_666_666_667,
            2
        );
        // x = 0.025 gives 1.025315104166..., against e^0.025 = 1.02531512.
        assert!(
            aave_math::calculate_compounded_interest(RAY / 40, 0, YEAR_MICROS)
                == 1_025_315_104_166_666_666_666_666_667,
            3
        );
        assert!(aave_math::calculate_compounded_interest(RAY / 10, 0, 0) == RAY, 4);
    }

    #[test]
    fun test_reserve_config_round_trips_at_boundaries() {
        let config = aave_config::init_reserve_configuration();
        aave_config::set_ltv(&mut config, 65535);
        aave_config::set_liquidation_threshold(&mut config, 65535);
        aave_config::set_liquidation_bonus(&mut config, 65535);
        aave_config::set_decimals(&mut config, 18);
        aave_config::set_reserve_factor(&mut config, 65535);
        aave_config::set_borrow_cap(&mut config, 68719476735);
        aave_config::set_supply_cap(&mut config, 68719476735);
        aave_config::set_liquidation_protocol_fee(&mut config, 65535);
        aave_config::set_active(&mut config, true);
        aave_config::set_frozen(&mut config, true);
        aave_config::set_borrowing_enabled(&mut config, true);
        aave_config::set_paused(&mut config, true);
        aave_config::set_flashloan_enabled(&mut config, true);

        assert!(aave_config::get_ltv(&config) == 65535, 1);
        assert!(aave_config::get_liquidation_threshold(&config) == 65535, 2);
        assert!(aave_config::get_liquidation_bonus(&config) == 65535, 3);
        assert!(aave_config::get_decimals(&config) == 18, 4);
        assert!(aave_config::get_reserve_factor(&config) == 65535, 5);
        assert!(aave_config::get_borrow_cap(&config) == 68719476735, 6);
        assert!(aave_config::get_supply_cap(&config) == 68719476735, 7);
        assert!(aave_config::get_liquidation_protocol_fee(&config) == 65535, 8);
        let (active, frozen, borrowing, paused, flashloan) =
            aave_config::get_flags(&config);
        assert!(active && frozen && borrowing && paused && flashloan, 9);

        // Clearing one field must leave every neighbouring field untouched.
        aave_config::set_decimals(&mut config, 6);
        aave_config::set_frozen(&mut config, false);
        assert!(aave_config::get_decimals(&config) == 6, 10);
        assert!(!aave_config::get_frozen(&config), 11);
        assert!(aave_config::get_ltv(&config) == 65535, 12);
        assert!(aave_config::get_liquidation_threshold(&config) == 65535, 13);
        assert!(aave_config::get_liquidation_bonus(&config) == 65535, 14);
        assert!(aave_config::get_reserve_factor(&config) == 65535, 15);
        assert!(aave_config::get_borrow_cap(&config) == 68719476735, 16);
        assert!(aave_config::get_supply_cap(&config) == 68719476735, 17);
        assert!(aave_config::get_liquidation_protocol_fee(&config) == 65535, 18);
        assert!(aave_config::get_active(&config), 19);
        assert!(aave_config::get_borrowing_enabled(&config), 20);
        assert!(aave_config::get_paused(&config), 21);
        assert!(aave_config::get_flashloan_enabled(&config), 22);

        aave_config::set_ltv(&mut config, 0);
        aave_config::set_liquidation_threshold(&mut config, 0);
        aave_config::set_liquidation_bonus(&mut config, 0);
        aave_config::set_reserve_factor(&mut config, 0);
        aave_config::set_borrow_cap(&mut config, 0);
        aave_config::set_supply_cap(&mut config, 0);
        aave_config::set_liquidation_protocol_fee(&mut config, 0);
        aave_config::set_active(&mut config, false);
        aave_config::set_borrowing_enabled(&mut config, false);
        aave_config::set_paused(&mut config, false);
        aave_config::set_flashloan_enabled(&mut config, false);
        aave_config::set_decimals(&mut config, 6);
        assert!(
            aave_config::reserve_configuration_data(&config)
                == (6 << 48),
            23
        );

        let restored =
            aave_config::reserve_configuration_from_data(
                aave_config::reserve_configuration_data(&config)
            );
        assert!(aave_config::get_decimals(&restored) == 6, 24);
    }

    #[test]
    #[expected_failure(abort_code = 13, location = bench::aave_config)]
    fun test_reserve_config_rejects_decimals_below_minimum() {
        let config = aave_config::init_reserve_configuration();
        aave_config::set_decimals(&mut config, 5);
    }

    #[test]
    fun test_user_config_bits_at_both_ends_of_the_range() {
        let config = aave_config::init_user_configuration();
        assert!(aave_config::is_empty(&config), 1);

        aave_config::set_borrowing(&mut config, 0, true);
        assert!(aave_config::user_configuration_data(&config) == 1, 2);
        assert!(aave_config::is_borrowing(&config, 0), 3);
        assert!(!aave_config::is_using_as_collateral(&config, 0), 4);
        assert!(aave_config::is_using_as_collateral_or_borrowing(&config, 0), 5);
        assert!(aave_config::is_borrowing_any(&config), 6);
        assert!(!aave_config::is_using_as_collateral_any(&config), 7);

        aave_config::set_using_as_collateral(&mut config, 0, true);
        assert!(aave_config::user_configuration_data(&config) == 3, 8);
        assert!(aave_config::is_using_as_collateral(&config, 0), 9);

        // Index 127 is the last reserve two bits per reserve leaves room for.
        aave_config::set_borrowing(&mut config, 127, true);
        aave_config::set_using_as_collateral(&mut config, 127, true);
        assert!(aave_config::is_borrowing(&config, 127), 10);
        assert!(aave_config::is_using_as_collateral(&config, 127), 11);
        assert!(
            aave_config::user_configuration_data(&config)
                == 3 + (1u256 << 254) + (1u256 << 255),
            12
        );
        assert!(!aave_config::is_borrowing(&config, 126), 13);
        assert!(!aave_config::is_using_as_collateral(&config, 126), 14);

        aave_config::set_borrowing(&mut config, 0, false);
        assert!(!aave_config::is_borrowing(&config, 0), 15);
        assert!(aave_config::is_using_as_collateral(&config, 0), 16);
        assert!(aave_config::is_borrowing_one(&config), 17);

        aave_config::set_using_as_collateral(&mut config, 127, false);
        assert!(aave_config::is_using_as_collateral_one(&config), 18);
        assert!(!aave_config::is_using_as_collateral(&config, 127), 19);
        assert!(aave_config::is_borrowing(&config, 127), 20);

        aave_config::set_borrowing(&mut config, 127, false);
        aave_config::set_using_as_collateral(&mut config, 0, false);
        assert!(aave_config::is_empty(&config), 21);
    }

    #[test]
    #[expected_failure(abort_code = 18, location = bench::aave_config)]
    fun test_user_config_rejects_index_past_the_range() {
        let config = aave_config::init_user_configuration();
        aave_config::set_borrowing(&mut config, 128, true);
    }

    #[test(aptos_framework = @0x1, admin = @bench, user = @0x123)]
    fun test_index_accrual_against_known_ray_values(
        aptos_framework: &signer, admin: &signer, user: &signer
    ) {
        start(aptos_framework, admin);
        let asset = add_reserve(admin, b"ACCR", 6, 100_000_000, 8000, 8500, 10500, 0, 0);
        let user_addr = signer::address_of(user);
        aave_mock_fa::mint(admin, asset, user_addr, 1_000_000);

        aave_logic::supply(user, asset, 1_000_000);
        aave_logic::set_user_use_reserve_as_collateral(user, asset, true);
        aave_logic::borrow(user, asset, 500_000, VARIABLE);

        // Half the liquidity is borrowed, so the two-slope model sits on the
        // first slope: 4% * 0.5 / 0.8 = 2.5% borrow, 2.5% * 0.5 = 1.25% supply.
        assert!(
            aave_pool::get_current_variable_borrow_rate(asset)
                == 25_000_000_000_000_000_000_000_000,
            1
        );
        assert!(
            aave_pool::get_current_liquidity_rate(asset)
                == 12_500_000_000_000_000_000_000_000,
            2
        );

        timestamp::fast_forward_seconds(YEAR);

        assert!(
            aave_pool::get_normalized_income(asset)
                == 1_012_500_000_000_000_000_000_000_000,
            3
        );
        assert!(
            aave_pool::get_normalized_debt(asset)
                == 1_025_315_104_166_666_666_666_666_667,
            4
        );

        let cache = aave_pool::cache(asset);
        aave_pool::update_state(&mut cache);
        assert!(
            aave_pool::get_liquidity_index(asset)
                == 1_012_500_000_000_000_000_000_000_000,
            5
        );
        assert!(
            aave_pool::get_variable_borrow_index(asset)
                == 1_025_315_104_166_666_666_666_666_667,
            6
        );

        // A scaled balance is untouched by accrual; only the index moved.
        assert!(scaled_collateral(user_addr, asset) == 1_000_000, 7);
        assert!(
            aave_tokens::balance_of(
                user_addr,
                aave_pool::get_a_token(asset),
                aave_pool::get_liquidity_index(asset)
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
        aave_mock_fa::mint(admin, a, user_addr, 1_000_000_000);
        aave_mock_fa::mint(admin, b, user_addr, 500_000_000);

        aave_logic::supply(user, a, 1_000_000_000);
        aave_logic::supply(user, b, 500_000_000);
        aave_logic::set_user_use_reserve_as_collateral(user, a, true);
        aave_logic::set_user_use_reserve_as_collateral(user, b, true);
        aave_logic::borrow(user, a, 400_000_000, VARIABLE);

        let (collateral, debt, ltv, threshold, hf, zero_ltv) =
            aave_logic::user_account_data(user_addr);
        assert!(collateral == 200_000_000_000, 1);
        assert!(debt == 40_000_000_000, 2);
        assert!(ltv == 6500, 3);
        assert!(threshold == 7250, 4);
        // 2000 * 0.725 / 400 = 3.625.
        assert!(hf == 3_625_000_000_000_000_000, 5);
        assert!(!zero_ltv, 6);

        assert!(
            aave_logic::calculate_available_borrows(collateral, debt, ltv)
                == 90_000_000_000,
            7
        );
    }

    #[test(aptos_framework = @0x1, admin = @bench, user = @0x123)]
    #[expected_failure(abort_code = 71, location = bench::aave_logic)]
    fun test_borrow_past_the_ltv_aborts(
        aptos_framework: &signer, admin: &signer, user: &signer
    ) {
        start(aptos_framework, admin);
        let a = add_reserve(admin, b"LTVA", 6, 100_000_000, 8000, 8500, 10500, 0, 0);
        let user_addr = signer::address_of(user);
        aave_mock_fa::mint(admin, a, user_addr, 1_000_000_000);

        aave_logic::supply(user, a, 1_000_000_000);
        aave_logic::set_user_use_reserve_as_collateral(user, a, true);
        // $1000 of collateral at 80% supports $800, not $900.
        aave_logic::borrow(user, a, 900_000_000, VARIABLE);
    }

    #[test(
        aptos_framework = @0x1, admin = @bench, user = @0x123, provider = @0x456
    )]
    #[expected_failure(abort_code = 70, location = bench::aave_logic)]
    fun test_borrow_below_health_factor_one_aborts(
        aptos_framework: &signer,
        admin: &signer,
        user: &signer,
        provider: &signer
    ) {
        start(aptos_framework, admin);
        let (collateral_asset, debt_asset, user_addr, _) =
            underwater_fixture(admin, user, provider);
        assert!(aave_logic::health_factor(user_addr) < WAD, 1);
        assert!(collateral_asset != debt_asset, 2);
        aave_logic::borrow(user, debt_asset, 1, VARIABLE);
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

        aave_mock_fa::mint(admin, collateral_asset, user_addr, 1_000_000_000);
        aave_mock_fa::mint(admin, debt_asset, provider_addr, 1_000_000_000);

        aave_logic::supply(provider, debt_asset, 1_000_000_000);
        aave_logic::supply(user, collateral_asset, 1_000_000_000);
        aave_logic::set_user_use_reserve_as_collateral(user, collateral_asset, true);
        aave_logic::borrow(user, debt_asset, 700_000_000, VARIABLE);
        assert!(
            aave_logic::health_factor(user_addr) == 1_214_285_714_285_714_286, 100
        );

        aave_pool::oracle_set_price(admin, collateral_asset, 70_000_000);
        assert!(aave_logic::health_factor(user_addr) == 850_000_000_000_000_000, 101);
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
        aave_mock_fa::mint(admin, debt_asset, liquidator_addr, 100_000_000);

        aave_logic::liquidation_call(
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
            aave_mock_fa::balance_of(liquidator_addr, collateral_asset) == 149_999_999,
            1
        );
        assert!(aave_mock_fa::balance_of(liquidator_addr, debt_asset) == 0, 2);
        assert!(scaled_collateral(user_addr, collateral_asset) == 850_000_001, 3);
        assert!(scaled_debt(user_addr, debt_asset) == 600_000_000, 4);
        // Below the break-even health factor of one over the bonus, closing
        // debt at a premium leaves the position slightly worse off.
        assert!(aave_logic::health_factor(user_addr) == 842_916_667_666_666_667, 5);

        let config = aave_pool::get_user_config(user_addr);
        assert!(
            aave_config::is_using_as_collateral(
                &config, (aave_pool::get_reserve_id(collateral_asset) as u256)
            ),
            6
        );
        assert!(
            aave_config::is_borrowing(
                &config, (aave_pool::get_reserve_id(debt_asset) as u256)
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
        aave_mock_fa::mint(admin, asset, provider_addr, 1_000_000_000);
        aave_mock_fa::mint(admin, asset, borrower_addr, 1_000_000);
        aave_logic::supply(provider, asset, 1_000_000_000);

        aave_logic::flash_loan_simple(borrower, asset, 100_000_000, 9, 64);

        // A 9 bps premium on 100 units is 0.09, paid out of the borrower's own
        // funds and folded into the index rather than minted.
        assert!(aave_mock_fa::balance_of(borrower_addr, asset) == 1_000_000 - 90_000, 1);
        assert!(aave_pool::get_liquidity_index(asset) > RAY, 2);
        assert!(
            aave_tokens::balance_of(
                provider_addr,
                aave_pool::get_a_token(asset),
                aave_pool::get_liquidity_index(asset)
            ) == 1_000_090_000,
            3
        );
        assert!(aave_pool::vault_balance(asset) == 1_000_090_000, 4);
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
        aave_mock_fa::mint(admin, asset, signer::address_of(provider), 1_000_000_000);
        aave_logic::supply(provider, asset, 1_000_000_000);

        // The borrower holds nothing beyond the loan itself, so the premium
        // cannot be paid.
        let receipt =
            aave_logic::flash_loan_simple_take(borrower, asset, 100_000_000, 9);
        aave_logic::flash_loan_simple_repay(borrower, receipt);
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
        assert!(aave_pool::reserves_count() == 4, 1);
        assert!(aave_pool::reserve_address_by_id(0) == r0, 2);
        assert!(aave_pool::reserve_address_by_id(3) == r3, 3);

        let addr1 = signer::address_of(user1);
        let addr2 = signer::address_of(user2);
        aave_mock_fa::mint(admin, r0, addr1, 10_000_000_000);
        aave_mock_fa::mint(admin, r1, addr1, 10_000_000_000);
        aave_mock_fa::mint(admin, r2, addr1, 1_000_000_000);
        aave_mock_fa::mint(admin, r2, addr2, 100_000_000);
        aave_mock_fa::mint(admin, r3, addr2, 10_000_000);

        // user1 posts $5000 and $1000; user2 posts $2000 and $3000.
        aave_logic::supply(user1, r0, 5_000_000_000);
        aave_logic::supply(user1, r1, 1_000_000_000);
        aave_logic::set_user_use_reserve_as_collateral(user1, r0, true);
        aave_logic::set_user_use_reserve_as_collateral(user1, r1, true);

        aave_logic::supply(user2, r2, 100_000_000);
        aave_logic::supply(user2, r3, 10_000_000);
        aave_logic::set_user_use_reserve_as_collateral(user2, r2, true);
        aave_logic::set_user_use_reserve_as_collateral(user2, r3, true);

        aave_logic::borrow(user2, r0, 2_000_000_000, VARIABLE);
        aave_logic::borrow(user1, r2, 10_000_000, VARIABLE);

        let (collateral2, debt2, _, _, hf2, _) = aave_logic::user_account_data(addr2);
        assert!(collateral2 == 500_000_000_000, 4);
        assert!(debt2 == 200_000_000_000, 5);
        assert!(hf2 == 2_125_000_000_000_000_000, 6);

        // Without elapsed time every index stays at one ray and the accrual
        // paths are never exercised.
        timestamp::fast_forward_seconds(MONTH);
        assert!(aave_pool::get_liquidity_index(r0) == RAY, 7);
        assert!(aave_pool::get_normalized_income(r0) > RAY, 8);
        assert!(aave_pool::get_normalized_debt(r0) > RAY, 9);

        aave_logic::supply(user1, r0, 1_000_000_000);
        assert!(aave_pool::get_liquidity_index(r0) > RAY, 10);
        assert!(aave_pool::get_variable_borrow_index(r0) > RAY, 11);
        assert!(aave_pool::get_accrued_to_treasury(r0) > 0, 12);

        aave_logic::withdraw(user1, r1, 500_000_000, addr1);
        aave_logic::borrow(user1, r2, 5_000_000, VARIABLE);
        aave_logic::repay(user1, r2, 5_000_000, VARIABLE);

        aave_logic::set_user_use_reserve_as_collateral(user1, r1, false);
        let config1 = aave_pool::get_user_config(addr1);
        assert!(
            !aave_config::is_using_as_collateral(
                &config1, (aave_pool::get_reserve_id(r1) as u256)
            ),
            13
        );
        aave_logic::set_user_use_reserve_as_collateral(user1, r1, true);

        aave_logic::flash_loan_simple(user1, r0, 1_000_000_000, 9, 128);

        // A 90% drop on user2's larger collateral takes them under water.
        aave_pool::oracle_set_price(admin, r3, 100_000_000_000);
        assert!(aave_logic::health_factor(addr2) < WAD, 14);

        let debt_before = scaled_debt(addr2, r0);
        let collateral_before = scaled_collateral(addr2, r2);
        let held_before = aave_mock_fa::balance_of(addr1, r2);
        aave_logic::liquidation_call(user1, r2, r0, addr2, 100_000_000, false);

        assert!(scaled_debt(addr2, r0) < debt_before, 15);
        assert!(scaled_collateral(addr2, r2) < collateral_before, 16);
        // $100 of debt buys $105 of collateral at $2000 a unit, less the 10%
        // protocol cut of the $5 bonus.
        assert!(aave_mock_fa::balance_of(addr1, r2) - held_before == 5_225_000, 17);
        assert!(
            aave_tokens::scaled_balance_of(@bench, aave_pool::get_a_token(r2)) > 0,
            18
        );
        assert!(aave_logic::health_factor(addr2) < WAD, 19);
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
            aave_mock_fa::mint(admin, asset, provider_addr, 10_000_000_000);
            aave_logic::supply(provider, asset, 10_000_000_000);
            r = r + 1;
        };

        assert!(!aave_pool::seeded_user_exists(42, 0), 1);
        aave_logic::seed_users(admin, 5, 2, 1_000_000_000, 500_000_000, 42);

        let i = 0;
        while (i < 5) {
            let user = aave_pool::seeded_user_address(42, i);
            assert!(aave_pool::seeded_user_exists(42, i), 2);

            // Two collaterals of $1000 at a 0.85 threshold carrying $500 of
            // debt: 0.85 * 2000 / 500 = 3.4.
            assert!(aave_logic::health_factor(user) == 3_400_000_000_000_000_000, 3);

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
    #[expected_failure(abort_code = 79, location = bench::aave_logic)]
    fun test_seed_users_needs_a_reserve_to_borrow_from(
        aptos_framework: &signer, admin: &signer
    ) {
        start(aptos_framework, admin);
        add_reserve(admin, b"SD10", 6, 100_000_000, 8000, 8500, 10500, 0, 0);
        add_reserve(admin, b"SD11", 6, 100_000_000, 8000, 8500, 10500, 0, 0);
        aave_logic::seed_users(admin, 1, 2, 1_000_000_000, 500_000_000, 7);
    }

    // The recipe a transaction harness follows: bootstrap liquidity once as
    // admin, onboard each account with one transaction, then run the mix. Every
    // step after onboarding must succeed from a plain signer.
    #[test(aptos_framework = @0x1, admin = @bench, user = @0xbe0)]
    fun test_harness_onboard_then_mix(
        aptos_framework: &signer, admin: &signer, user: &signer
    ) {
        start(aptos_framework, admin);
        let assets = vector[
            add_reserve(admin, b"HM0", 6, 100_000_000, 8000, 8500, 10500, 1000, 0),
            add_reserve(admin, b"HM1", 6, 100_000_000, 8000, 8500, 10500, 1000, 0),
            add_reserve(admin, b"HM2", 6, 200_000_000, 7500, 8000, 11000, 1000, 0),
            add_reserve(admin, b"HM3", 6, 50_000_000, 7000, 7500, 11000, 1000, 0)
        ];
        // Every reserve needs liquidity before anyone can borrow from it, and
        // `seed_users` only ever covers a window of the reserve space.
        let r = 0;
        while (r < 4) {
            aave_logic::bench_supply(admin, *vector::borrow(&assets, r), 100_000_000_000);
            r = r + 1;
        };

        aave_logic::bench_onboard(user, 1, 2, 1_000_000_000, 100_000_000);
        let user_addr = signer::address_of(user);
        assert!(scaled_collateral(user_addr, *vector::borrow(&assets, 1)) > 0, 1);
        assert!(scaled_collateral(user_addr, *vector::borrow(&assets, 2)) > 0, 2);
        assert!(scaled_debt(user_addr, *vector::borrow(&assets, 3)) > 0, 3);

        let collateral = *vector::borrow(&assets, 1);
        let debt = *vector::borrow(&assets, 3);
        aave_logic::bench_supply(user, collateral, 1_000_000);
        aave_logic::withdraw(user, collateral, 1_000_000, user_addr);
        aave_logic::borrow(user, debt, 1_000_000, VARIABLE);
        aave_logic::repay(user, debt, 1_000_000, VARIABLE);
        aave_logic::set_user_use_reserve_as_collateral(user, collateral, true);
        aave_logic::flash_loan_simple(user, collateral, 10_000_000, 9, 4);

        // The mix is net-neutral, so the position it started from survives it.
        assert!(scaled_debt(user_addr, debt) > 0, 4);
        assert!(scaled_collateral(user_addr, collateral) > 0, 5);
    }
}
