// Ported from https://github.com/aave/aptos-aave-v3, modules
// `aave_pool::generic_logic`, `aave_pool::validation_logic`,
// `aave_pool::supply_logic`, `aave_pool::borrow_logic`,
// `aave_pool::liquidation_logic` and `aave_pool::flashloan_logic`.
// Copyright (c) Aave DAO and contributors.
// SPDX-License-Identifier: Apache-2.0

/// User-facing actions, the health-factor computation and the validations.
///
/// `calculate_user_account_data` is the read-set generator: it walks the
/// user's two-bit-per-reserve config and, for every set bit, reads that
/// reserve's group, its oracle price and the user's scaled balances. Two calls
/// to the same entry function therefore touch very different amounts of state
/// depending on how many reserves the caller is in.
module bench::aave_logic {
    use std::signer;
    use bench::aave_config::{Self, ReserveConfigurationMap, UserConfigurationMap};
    use bench::aave_math;
    use bench::aave_mock_fa;
    use bench::aave_pool::{Self, ReserveCache};
    use bench::aave_tokens;

    const LCG_MUL: u64 = 1103515245;
    const LCG_INC: u64 = 12345;
    const LCG_MOD: u64 = 1000003;

    /// Half the debt may be closed in one call once the position is only
    /// mildly unhealthy.
    const DEFAULT_LIQUIDATION_CLOSE_FACTOR: u256 = 5000;
    const CLOSE_FACTOR_HF_THRESHOLD: u256 = 950_000_000_000_000_000;
    /// Positions above this size in base currency are subject to the close
    /// factor; smaller ones may be closed in full.
    const MIN_BASE_MAX_CLOSE_FACTOR_THRESHOLD: u256 = 500_000_000_000_000_000_000;

    const EAMOUNT_ZERO: u64 = 60;
    const ERESERVE_INACTIVE: u64 = 61;
    const ERESERVE_PAUSED: u64 = 62;
    const ERESERVE_FROZEN: u64 = 63;
    const EBORROWING_DISABLED: u64 = 64;
    const EFLASHLOAN_DISABLED: u64 = 65;
    const EINVALID_INTEREST_RATE_MODE: u64 = 66;
    const ENOT_ENOUGH_BALANCE: u64 = 67;
    const ECOLLATERAL_BALANCE_ZERO: u64 = 68;
    const ELTV_ZERO: u64 = 69;
    const EHEALTH_FACTOR_TOO_LOW: u64 = 70;
    const ECOLLATERAL_CANNOT_COVER_BORROW: u64 = 71;
    const ENO_DEBT_OF_SELECTED_TYPE: u64 = 72;
    const EHEALTH_FACTOR_NOT_BELOW_THRESHOLD: u64 = 73;
    const ECOLLATERAL_CANNOT_BE_LIQUIDATED: u64 = 74;
    const ESUPPLY_CAP_EXCEEDED: u64 = 75;
    const EBORROW_CAP_EXCEEDED: u64 = 76;
    const EUNDERLYING_BALANCE_ZERO: u64 = 77;
    const EINCONSISTENT_FLASHLOAN_PARAMS: u64 = 78;
    const ENOT_ENOUGH_RESERVES: u64 = 79;

    /// No abilities, so the compiler forces every borrowed amount back through
    /// `flash_loan_simple_repay` before the transaction can end.
    struct FlashLoanReceipt {
        asset: address,
        amount: u256,
        premium: u256,
        receiver: address
    }

    public entry fun supply(
        account: &signer, asset: address, amount: u256
    ) {
        let user = signer::address_of(account);
        let cache = aave_pool::cache(asset);
        aave_pool::update_state(&mut cache);
        validate_supply(&cache, amount);
        aave_pool::update_interest_rates_and_virtual_balance(&cache, amount, 0);
        aave_pool::vault_deposit(account, &cache, (amount as u64));
        aave_tokens::mint_scaled(
            user,
            amount,
            aave_pool::cache_next_liquidity_index(&cache),
            aave_pool::cache_a_token(&cache),
            false
        );
    }

    public entry fun withdraw(
        account: &signer,
        asset: address,
        amount: u256,
        to: address
    ) {
        let user = signer::address_of(account);
        let reserves_count = aave_pool::reserves_count();
        let cache = aave_pool::cache(asset);
        aave_pool::update_state(&mut cache);

        let a_token = aave_pool::cache_a_token(&cache);
        let next_liquidity_index = aave_pool::cache_next_liquidity_index(&cache);
        let user_balance =
            aave_math::ray_mul_down(
                aave_tokens::scaled_balance_of(user, a_token), next_liquidity_index
            );
        let amount_to_withdraw =
            if (amount == aave_math::u256_max()) { user_balance } else { amount };

        validate_withdraw(&cache, amount_to_withdraw, user_balance);
        aave_pool::update_interest_rates_and_virtual_balance(
            &cache, 0, amount_to_withdraw
        );

        let user_config = aave_pool::get_user_config(user);
        let reserve_id = (aave_pool::cache_id(&cache) as u256);
        let is_collateral =
            aave_config::is_using_as_collateral(&user_config, reserve_id);
        if (is_collateral && amount_to_withdraw == user_balance) {
            aave_config::set_using_as_collateral(&mut user_config, reserve_id, false);
            aave_pool::set_user_config(user, &user_config);
        };

        aave_tokens::burn_scaled(
            user, amount_to_withdraw, next_liquidity_index, a_token, true
        );
        aave_pool::vault_withdraw(&cache, to, (amount_to_withdraw as u64));

        if (is_collateral && aave_config::is_borrowing_any(&user_config)) {
            validate_hf_and_ltv(&user_config, user, reserves_count);
        }
    }

    public entry fun borrow(
        account: &signer,
        asset: address,
        amount: u256,
        interest_rate_mode: u8
    ) {
        let user = signer::address_of(account);
        let reserves_count = aave_pool::reserves_count();
        let cache = aave_pool::cache(asset);
        aave_pool::update_state(&mut cache);

        let user_config = aave_pool::get_user_config(user);
        validate_borrow(
            &cache,
            &user_config,
            user,
            amount,
            interest_rate_mode,
            reserves_count
        );

        let debt_token = aave_pool::cache_variable_debt_token(&cache);
        let is_first_borrowing =
            aave_tokens::mint_scaled(
                user,
                amount,
                aave_pool::cache_next_variable_borrow_index(&cache),
                debt_token,
                true
            );
        aave_pool::cache_set_next_scaled_variable_debt(
            &mut cache, aave_tokens::scaled_total_supply(debt_token)
        );

        if (is_first_borrowing) {
            aave_config::set_borrowing(
                &mut user_config, (aave_pool::cache_id(&cache) as u256), true
            );
            aave_pool::set_user_config(user, &user_config);
        };

        aave_pool::update_interest_rates_and_virtual_balance(&cache, 0, amount);
        aave_pool::vault_withdraw(&cache, user, (amount as u64));
    }

    public entry fun repay(
        account: &signer,
        asset: address,
        amount: u256,
        interest_rate_mode: u8
    ) {
        let user = signer::address_of(account);
        let cache = aave_pool::cache(asset);
        aave_pool::update_state(&mut cache);

        let debt_token = aave_pool::cache_variable_debt_token(&cache);
        let next_borrow_index = aave_pool::cache_next_variable_borrow_index(&cache);
        let variable_debt =
            aave_math::ray_mul(
                aave_tokens::scaled_balance_of(user, debt_token), next_borrow_index
            );
        validate_repay(&cache, amount, interest_rate_mode, variable_debt);

        let payback_amount = if (amount < variable_debt) { amount } else {
            variable_debt
        };

        aave_tokens::burn_scaled(
            user, payback_amount, next_borrow_index, debt_token, false
        );
        aave_pool::cache_set_next_scaled_variable_debt(
            &mut cache, aave_tokens::scaled_total_supply(debt_token)
        );
        aave_pool::update_interest_rates_and_virtual_balance(&cache, payback_amount, 0);

        if (variable_debt - payback_amount == 0) {
            let user_config = aave_pool::get_user_config(user);
            aave_config::set_borrowing(
                &mut user_config, (aave_pool::cache_id(&cache) as u256), false
            );
            aave_pool::set_user_config(user, &user_config);
        };

        aave_pool::vault_deposit(account, &cache, (payback_amount as u64));
    }

    public entry fun set_user_use_reserve_as_collateral(
        account: &signer, asset: address, use_as_collateral: bool
    ) {
        let user = signer::address_of(account);
        let reserves_count = aave_pool::reserves_count();
        let cache = aave_pool::cache(asset);
        let user_balance =
            aave_tokens::balance_of(
                user,
                aave_pool::cache_a_token(&cache),
                aave_pool::cache_next_liquidity_index(&cache)
            );
        let user_config = aave_pool::get_user_config(user);
        let reserve_id = (aave_pool::cache_id(&cache) as u256);
        let is_collateral =
            aave_config::is_using_as_collateral(&user_config, reserve_id);

        validate_set_use_reserve_as_collateral(&cache, user_balance);
        if (use_as_collateral == is_collateral) {
            return
        };

        if (use_as_collateral) {
            let config = aave_pool::cache_configuration(&cache);
            assert!(aave_config::get_ltv(&config) != 0, ELTV_ZERO);
            aave_config::set_using_as_collateral(&mut user_config, reserve_id, true);
            aave_pool::set_user_config(user, &user_config);
        } else {
            aave_config::set_using_as_collateral(&mut user_config, reserve_id, false);
            aave_pool::set_user_config(user, &user_config);
            validate_hf_and_ltv(&user_config, user, reserves_count);
        }
    }

    public entry fun liquidation_call(
        account: &signer,
        collateral_asset: address,
        debt_asset: address,
        user: address,
        debt_to_cover: u256,
        receive_a_token: bool
    ) {
        let liquidator = signer::address_of(account);
        let reserves_count = aave_pool::reserves_count();
        let debt_cache = aave_pool::cache(debt_asset);
        aave_pool::update_state(&mut debt_cache);

        let user_config = aave_pool::get_user_config(user);
        let (_, total_debt_base, _, _, health_factor, _) =
            calculate_user_account_data(&user_config, reserves_count, user);

        let collateral_config = aave_pool::get_configuration(collateral_asset);
        let collateral_a_token = aave_pool::get_a_token(collateral_asset);
        let collateral_income = aave_pool::get_normalized_income(collateral_asset);
        let user_collateral_balance =
            aave_math::ray_mul(
                aave_tokens::scaled_balance_of(user, collateral_a_token),
                collateral_income
            );

        let debt_token = aave_pool::cache_variable_debt_token(&debt_cache);
        let next_borrow_index =
            aave_pool::cache_next_variable_borrow_index(&debt_cache);
        let user_reserve_debt =
            aave_math::ray_mul(
                aave_tokens::scaled_balance_of(user, debt_token), next_borrow_index
            );

        validate_liquidation_call(
            &user_config,
            &collateral_config,
            &debt_cache,
            (aave_pool::get_reserve_id(collateral_asset) as u256),
            user_reserve_debt,
            health_factor
        );

        let liquidation_bonus = aave_config::get_liquidation_bonus(&collateral_config);
        let collateral_price = aave_pool::asset_price(collateral_asset);
        let debt_price = aave_pool::asset_price(debt_asset);
        let collateral_unit =
            aave_math::pow(10, aave_config::get_decimals(&collateral_config));
        let debt_unit = aave_pool::cache_asset_unit(&debt_cache);

        let user_debt_base =
            aave_math::ceil_div(user_reserve_debt * debt_price, debt_unit);
        let user_collateral_base =
            (user_collateral_balance * collateral_price) / collateral_unit;

        let max_liquidatable_debt = user_reserve_debt;
        if (user_collateral_base >= MIN_BASE_MAX_CLOSE_FACTOR_THRESHOLD
            && user_debt_base >= MIN_BASE_MAX_CLOSE_FACTOR_THRESHOLD
            && health_factor > CLOSE_FACTOR_HF_THRESHOLD) {
            let default_liquidatable_base =
                aave_math::percent_mul(
                    total_debt_base, DEFAULT_LIQUIDATION_CLOSE_FACTOR
                );
            if (user_debt_base > default_liquidatable_base) {
                max_liquidatable_debt = (default_liquidatable_base * debt_unit)
                    / debt_price;
            }
        };

        let requested_debt =
            if (debt_to_cover > max_liquidatable_debt) {
                max_liquidatable_debt
            } else { debt_to_cover };

        let (collateral_to_liquidate, actual_debt_to_liquidate, protocol_fee, _) =
            calculate_available_collateral_to_liquidate(
                &collateral_config,
                collateral_price,
                collateral_unit,
                debt_price,
                debt_unit,
                requested_debt,
                user_collateral_balance,
                liquidation_bonus
            );

        if (collateral_to_liquidate + protocol_fee == user_collateral_balance) {
            aave_config::set_using_as_collateral(
                &mut user_config,
                (aave_pool::get_reserve_id(collateral_asset) as u256),
                false
            );
            aave_pool::set_user_config(user, &user_config);
        };

        aave_tokens::burn_scaled(
            user, actual_debt_to_liquidate, next_borrow_index, debt_token, false
        );
        aave_pool::cache_set_next_scaled_variable_debt(
            &mut debt_cache, aave_tokens::scaled_total_supply(debt_token)
        );
        if (user_reserve_debt - actual_debt_to_liquidate == 0) {
            aave_config::set_borrowing(
                &mut user_config, (aave_pool::cache_id(&debt_cache) as u256), false
            );
            aave_pool::set_user_config(user, &user_config);
        };

        if (receive_a_token) {
            aave_tokens::transfer_scaled(
                user,
                liquidator,
                collateral_to_liquidate,
                collateral_income,
                collateral_a_token
            );
            if (protocol_fee != 0) {
                aave_tokens::transfer_scaled(
                    user,
                    @bench,
                    protocol_fee,
                    collateral_income,
                    collateral_a_token
                );
            }
        } else {
            let collateral_cache = aave_pool::cache(collateral_asset);
            aave_pool::update_state(&mut collateral_cache);
            aave_pool::update_interest_rates_and_virtual_balance(
                &collateral_cache, 0, collateral_to_liquidate
            );
            let next_liquidity_index =
                aave_pool::cache_next_liquidity_index(&collateral_cache);
            aave_tokens::burn_scaled(
                user,
                collateral_to_liquidate,
                next_liquidity_index,
                collateral_a_token,
                true
            );
            aave_pool::vault_withdraw(
                &collateral_cache, liquidator, (collateral_to_liquidate as u64)
            );
            if (protocol_fee != 0) {
                aave_tokens::transfer_scaled(
                    user,
                    @bench,
                    protocol_fee,
                    next_liquidity_index,
                    collateral_a_token
                );
            }
        };

        aave_pool::update_interest_rates_and_virtual_balance(
            &debt_cache, actual_debt_to_liquidate, 0
        );
        aave_pool::vault_deposit(
            account, &debt_cache, (actual_debt_to_liquidate as u64)
        );
    }

    /// Borrows, runs `receiver_ops` iterations of the benchmark receiver, then
    /// repays with a premium, all in one transaction.
    public entry fun flash_loan_simple(
        account: &signer,
        asset: address,
        amount: u256,
        premium_bps: u256,
        receiver_ops: u64
    ) {
        let receipt = flash_loan_simple_take(account, asset, amount, premium_bps);
        flash_loan_receiver_work(receiver_ops);
        flash_loan_simple_repay(account, receipt);
    }

    public fun flash_loan_simple_take(
        account: &signer,
        asset: address,
        amount: u256,
        premium_bps: u256
    ): FlashLoanReceipt {
        let receiver = signer::address_of(account);
        let cache = aave_pool::cache(asset);
        aave_pool::update_state(&mut cache);
        validate_flashloan_simple(&cache, amount);

        let premium = aave_math::percent_mul(amount, premium_bps);
        aave_pool::update_interest_rates_and_virtual_balance(&cache, 0, amount);
        aave_pool::vault_withdraw(&cache, receiver, (amount as u64));
        FlashLoanReceipt { asset, amount, premium, receiver }
    }

    public fun flash_loan_simple_repay(
        account: &signer, receipt: FlashLoanReceipt
    ) {
        let FlashLoanReceipt { asset, amount, premium, receiver } = receipt;
        assert!(
            signer::address_of(account) == receiver, EINCONSISTENT_FLASHLOAN_PARAMS
        );
        let cache = aave_pool::cache(asset);
        aave_pool::update_state(&mut cache);

        if (premium != 0) {
            let total_liquidity =
                aave_tokens::total_supply(
                    aave_pool::cache_a_token(&cache),
                    aave_pool::cache_next_liquidity_index(&cache)
                );
            aave_pool::cumulate_to_liquidity_index(&mut cache, total_liquidity, premium);
        };
        aave_pool::update_interest_rates_and_virtual_balance(
            &cache, amount + premium, 0
        );
        aave_pool::vault_deposit(account, &cache, ((amount + premium) as u64));
    }

    /// Stands in for the borrower's own logic between take and repay, so a
    /// flash loan costs more than two state writes.
    public fun flash_loan_receiver_work(ops: u64): u64 {
        let acc = 1;
        let i = 0;
        while (i < ops) {
            acc = (acc * LCG_MUL + LCG_INC) % LCG_MOD;
            i = i + 1;
        };
        acc
    }

    /// Builds `n_users` synthetic users in one transaction. Each one supplies
    /// `supply_amount` on `collaterals_per_user` reserves, enables all of them
    /// as collateral, and borrows `borrow_amount` on one further reserve.
    ///
    /// The reserves are a window of the id space starting where the LCG lands,
    /// so users overlap without every user touching the same reserves. Every
    /// supply runs before any borrow, which keeps a borrow from arriving at a
    /// reserve whose only supplier is a later user.
    ///
    /// Brings a fresh account to the state the benchmark mix assumes: funded,
    /// supplying `collaterals` reserves starting at `start`, and carrying one
    /// variable-rate debt position on the next reserve after those.
    ///
    /// `start` is an argument rather than derived from the address so a
    /// harness can spread accounts over the reserve space however it likes.
    public entry fun bench_onboard(
        account: &signer,
        start: u64,
        collaterals: u64,
        supply_amount: u256,
        borrow_amount: u256
    ) {
        assert!(collaterals > 0, EAMOUNT_ZERO);
        let n_reserves = aave_pool::reserves_count();
        assert!(n_reserves > collaterals, ENOT_ENOUGH_RESERVES);

        let user = signer::address_of(account);
        let k = 0;
        while (k < collaterals) {
            let asset = aave_pool::reserve_address_by_id((start + k) % n_reserves);
            // Twice what is supplied, so the account keeps a spendable balance
            // for flash-loan premiums and repays.
            aave_mock_fa::faucet(asset, user, ((supply_amount * 2) as u64));
            supply(account, asset, supply_amount);
            set_user_use_reserve_as_collateral(account, asset, true);
            k = k + 1;
        };

        // Fund the debt asset too, so later repays and flash-loan premiums do
        // not depend on the account still holding what it borrowed.
        let debt_asset =
            aave_pool::reserve_address_by_id((start + collaterals) % n_reserves);
        aave_mock_fa::faucet(debt_asset, user, (supply_amount as u64));
        borrow(
            account,
            debt_asset,
            borrow_amount,
            aave_config::interest_rate_mode_variable()
        );
    }

    /// Faucet then supply, so a benchmark account never runs out of underlying
    /// however long the mix runs.
    public entry fun bench_supply(
        account: &signer, asset: address, amount: u256
    ) {
        aave_mock_fa::faucet(asset, signer::address_of(account), (amount as u64));
        supply(account, asset, amount);
    }

    /// Each borrow reserve still needs liquidity from somewhere. With few
    /// users the LCG will not cover the id space, so seed a liquidity provider
    /// across all reserves first.
    public entry fun seed_users(
        admin: &signer,
        n_users: u64,
        collaterals_per_user: u64,
        supply_amount: u256,
        borrow_amount: u256,
        seed: u64
    ) {
        assert!(collaterals_per_user > 0, EAMOUNT_ZERO);
        let n_reserves = aave_pool::reserves_count();
        assert!(n_reserves > collaterals_per_user, ENOT_ENOUGH_RESERVES);

        let state = seed % LCG_MOD;
        let i = 0;
        while (i < n_users) {
            state = (state * LCG_MUL + LCG_INC) % LCG_MOD;
            let start = state % n_reserves;
            let user = aave_pool::seeded_user_signer(admin, seed, i);
            let user_addr = signer::address_of(&user);
            let k = 0;
            while (k < collaterals_per_user) {
                let asset = aave_pool::reserve_address_by_id((start + k) % n_reserves);
                aave_mock_fa::mint(admin, asset, user_addr, (supply_amount as u64));
                supply(&user, asset, supply_amount);
                set_user_use_reserve_as_collateral(&user, asset, true);
                k = k + 1;
            };
            i = i + 1;
        };

        // Same seed, same sequence, so this pass sees each user's own window.
        state = seed % LCG_MOD;
        i = 0;
        while (i < n_users) {
            state = (state * LCG_MUL + LCG_INC) % LCG_MOD;
            let borrow_id =
                (state % n_reserves + collaterals_per_user) % n_reserves;
            let user = aave_pool::seeded_user_signer(admin, seed, i);
            borrow(
                &user,
                aave_pool::reserve_address_by_id(borrow_id),
                borrow_amount,
                aave_config::interest_rate_mode_variable()
            );
            i = i + 1;
        };
    }

    /// Walks the user's config bitmask once, accumulating collateral, debt and
    /// the collateral-weighted LTV and liquidation threshold.
    ///
    /// Returns total collateral, total debt, average LTV, average liquidation
    /// threshold, health factor and whether any zero-LTV collateral was seen.
    /// All base-currency values carry the oracle's own decimals.
    public fun calculate_user_account_data(
        user_config: &UserConfigurationMap, reserves_count: u64, user: address
    ): (u256, u256, u256, u256, u256, bool) {
        if (aave_config::is_empty(user_config)) {
            return (0, 0, 0, 0, aave_math::u256_max(), false)
        };

        let total_collateral_base = 0;
        let total_debt_base = 0;
        let average_ltv = 0;
        let average_liquidation_threshold = 0;
        let has_zero_ltv_collateral = false;

        let i = 0;
        while (i < reserves_count) {
            let index = (i as u256);
            if (!aave_config::is_using_as_collateral_or_borrowing(user_config, index)) {
                i = i + 1;
                continue
            };

            let underlying = aave_pool::reserve_address_by_id(i);
            let view = aave_pool::account_view(underlying);
            let asset_price = aave_pool::asset_price(underlying);
            let asset_unit = aave_pool::view_asset_unit(&view);
            let liquidation_threshold = aave_pool::view_liquidation_threshold(&view);

            if (liquidation_threshold != 0
                && aave_config::is_using_as_collateral(user_config, index)) {
                let balance_base =
                    aave_math::ray_mul_down(
                        aave_tokens::scaled_balance_of(
                            user, aave_pool::view_a_token(&view)
                        ),
                        aave_pool::view_normalized_income(&view)
                    ) * asset_price / asset_unit;
                total_collateral_base = total_collateral_base + balance_base;

                let ltv = aave_pool::view_ltv(&view);
                if (ltv != 0) {
                    average_ltv = average_ltv + balance_base * ltv;
                } else {
                    has_zero_ltv_collateral = true;
                };
                average_liquidation_threshold = average_liquidation_threshold
                    + balance_base * liquidation_threshold;
            };

            if (aave_config::is_borrowing(user_config, index)) {
                let debt =
                    aave_tokens::scaled_balance_of(
                        user, aave_pool::view_variable_debt_token(&view)
                    );
                if (debt != 0) {
                    debt = aave_math::ray_mul_up(
                        debt, aave_pool::view_normalized_debt(&view)
                    );
                };
                total_debt_base = total_debt_base
                    + aave_math::ceil_div(asset_price * debt, asset_unit);
            };

            i = i + 1;
        };

        if (total_collateral_base != 0) {
            average_ltv = average_ltv / total_collateral_base;
            average_liquidation_threshold = average_liquidation_threshold
                / total_collateral_base;
        } else {
            average_ltv = 0;
            average_liquidation_threshold = 0;
        };

        let health_factor =
            if (total_debt_base == 0) {
                aave_math::u256_max()
            } else {
                aave_math::wad_div(
                    aave_math::percent_mul(
                        total_collateral_base, average_liquidation_threshold
                    ),
                    total_debt_base
                )
            };

        (
            total_collateral_base,
            total_debt_base,
            average_ltv,
            average_liquidation_threshold,
            health_factor,
            has_zero_ltv_collateral
        )
    }

    public fun calculate_available_borrows(
        total_collateral_base: u256, total_debt_base: u256, ltv: u256
    ): u256 {
        let available = aave_math::percent_mul(total_collateral_base, ltv);
        if (available <= total_debt_base) {
            return 0
        };
        available - total_debt_base
    }

    /// Splits `debt_to_cover` into the collateral the liquidator receives, the
    /// debt actually repaid, the protocol's cut of the bonus, and the seized
    /// collateral in base currency.
    public fun calculate_available_collateral_to_liquidate(
        collateral_config: &ReserveConfigurationMap,
        collateral_price: u256,
        collateral_unit: u256,
        debt_price: u256,
        debt_unit: u256,
        debt_to_cover: u256,
        user_collateral_balance: u256,
        liquidation_bonus: u256
    ): (u256, u256, u256, u256) {
        let fee_percentage =
            aave_config::get_liquidation_protocol_fee(collateral_config);
        let base_collateral =
            (debt_price * debt_to_cover * collateral_unit)
                / (collateral_price * debt_unit);
        let max_collateral_to_liquidate =
            aave_math::percent_mul(base_collateral, liquidation_bonus);

        let collateral_amount;
        let debt_amount_needed;
        if (max_collateral_to_liquidate > user_collateral_balance) {
            collateral_amount = user_collateral_balance;
            debt_amount_needed = aave_math::percent_div(
                (collateral_price * collateral_amount * debt_unit)
                    / (debt_price * collateral_unit),
                liquidation_bonus
            );
        } else {
            collateral_amount = max_collateral_to_liquidate;
            debt_amount_needed = debt_to_cover;
        };

        let collateral_base = (collateral_amount * collateral_price) / collateral_unit;

        let protocol_fee = 0;
        if (fee_percentage != 0) {
            let bonus_collateral =
                collateral_amount
                    - aave_math::percent_div(collateral_amount, liquidation_bonus);
            protocol_fee = aave_math::percent_mul(bonus_collateral, fee_percentage);
            collateral_amount = collateral_amount - protocol_fee;
        };

        (collateral_amount, debt_amount_needed, protocol_fee, collateral_base)
    }

    public fun validate_supply(cache: &ReserveCache, amount: u256) {
        assert!(amount != 0, EAMOUNT_ZERO);
        let config = aave_pool::cache_configuration(cache);
        let (active, frozen, _, paused, _) = aave_config::get_flags(&config);
        assert!(active, ERESERVE_INACTIVE);
        assert!(!paused, ERESERVE_PAUSED);
        assert!(!frozen, ERESERVE_FROZEN);

        let supply_cap = aave_config::get_supply_cap(&config);
        if (supply_cap != 0) {
            let a_token = aave_pool::cache_a_token(cache);
            let supply =
                aave_math::ray_mul(
                    aave_tokens::scaled_total_supply(a_token)
                        + aave_pool::get_accrued_to_treasury(
                            aave_pool::cache_underlying(cache)
                        ),
                    aave_pool::cache_next_liquidity_index(cache)
                );
            assert!(
                supply + amount
                    <= supply_cap * aave_pool::cache_asset_unit(cache),
                ESUPPLY_CAP_EXCEEDED
            );
        }
    }

    public fun validate_withdraw(
        cache: &ReserveCache, amount: u256, user_balance: u256
    ) {
        assert!(amount != 0, EAMOUNT_ZERO);
        assert!(amount <= user_balance, ENOT_ENOUGH_BALANCE);
        let config = aave_pool::cache_configuration(cache);
        let (active, _, _, paused, _) = aave_config::get_flags(&config);
        assert!(active, ERESERVE_INACTIVE);
        assert!(!paused, ERESERVE_PAUSED);
    }

    public fun validate_borrow(
        cache: &ReserveCache,
        user_config: &UserConfigurationMap,
        user: address,
        amount: u256,
        interest_rate_mode: u8,
        reserves_count: u64
    ) {
        assert!(amount != 0, EAMOUNT_ZERO);
        let config = aave_pool::cache_configuration(cache);
        let (active, frozen, borrowing_enabled, paused, _) =
            aave_config::get_flags(&config);
        assert!(active, ERESERVE_INACTIVE);
        assert!(!paused, ERESERVE_PAUSED);
        assert!(!frozen, ERESERVE_FROZEN);
        assert!(borrowing_enabled, EBORROWING_DISABLED);
        assert!(
            interest_rate_mode == aave_config::interest_rate_mode_variable(),
            EINVALID_INTEREST_RATE_MODE
        );

        let borrow_cap = aave_config::get_borrow_cap(&config);
        if (borrow_cap != 0) {
            let total_debt =
                aave_math::ray_mul(
                    aave_pool::cache_next_scaled_variable_debt(cache),
                    aave_pool::cache_next_variable_borrow_index(cache)
                );
            assert!(
                total_debt + amount
                    <= borrow_cap * aave_pool::cache_asset_unit(cache),
                EBORROW_CAP_EXCEEDED
            );
        };

        let (total_collateral_base, total_debt_base, current_ltv, _, health_factor, _) =
            calculate_user_account_data(user_config, reserves_count, user);
        assert!(total_collateral_base != 0, ECOLLATERAL_BALANCE_ZERO);
        assert!(current_ltv != 0, ELTV_ZERO);
        assert!(
            health_factor >= aave_config::health_factor_liquidation_threshold(),
            EHEALTH_FACTOR_TOO_LOW
        );

        let amount_base =
            (aave_pool::asset_price(aave_pool::cache_underlying(cache)) * amount)
                / aave_pool::cache_asset_unit(cache);
        assert!(
            aave_math::percent_div(total_debt_base + amount_base, current_ltv)
                <= total_collateral_base,
            ECOLLATERAL_CANNOT_COVER_BORROW
        );
    }

    public fun validate_repay(
        cache: &ReserveCache,
        amount: u256,
        interest_rate_mode: u8,
        variable_debt: u256
    ) {
        assert!(amount != 0, EAMOUNT_ZERO);
        assert!(
            interest_rate_mode == aave_config::interest_rate_mode_variable(),
            EINVALID_INTEREST_RATE_MODE
        );
        assert!(variable_debt != 0, ENO_DEBT_OF_SELECTED_TYPE);
        let config = aave_pool::cache_configuration(cache);
        let (active, _, _, paused, _) = aave_config::get_flags(&config);
        assert!(active, ERESERVE_INACTIVE);
        assert!(!paused, ERESERVE_PAUSED);
    }

    public fun validate_set_use_reserve_as_collateral(
        cache: &ReserveCache, user_balance: u256
    ) {
        assert!(user_balance != 0, EUNDERLYING_BALANCE_ZERO);
        let config = aave_pool::cache_configuration(cache);
        let (active, _, _, paused, _) = aave_config::get_flags(&config);
        assert!(active, ERESERVE_INACTIVE);
        assert!(!paused, ERESERVE_PAUSED);
    }

    public fun validate_liquidation_call(
        user_config: &UserConfigurationMap,
        collateral_config: &ReserveConfigurationMap,
        debt_cache: &ReserveCache,
        collateral_reserve_id: u256,
        user_reserve_debt: u256,
        health_factor: u256
    ) {
        let debt_config = aave_pool::cache_configuration(debt_cache);
        let (collateral_active, _, _, collateral_paused, _) =
            aave_config::get_flags(collateral_config);
        let (debt_active, _, _, debt_paused, _) = aave_config::get_flags(&debt_config);
        assert!(collateral_active && debt_active, ERESERVE_INACTIVE);
        assert!(!collateral_paused && !debt_paused, ERESERVE_PAUSED);
        assert!(
            health_factor < aave_config::health_factor_liquidation_threshold(),
            EHEALTH_FACTOR_NOT_BELOW_THRESHOLD
        );
        assert!(
            aave_config::get_liquidation_threshold(collateral_config) != 0
                && aave_config::is_using_as_collateral(
                    user_config, collateral_reserve_id
                ),
            ECOLLATERAL_CANNOT_BE_LIQUIDATED
        );
        assert!(user_reserve_debt != 0, ENO_DEBT_OF_SELECTED_TYPE);
    }

    public fun validate_flashloan_simple(cache: &ReserveCache, amount: u256) {
        assert!(amount != 0, EAMOUNT_ZERO);
        let config = aave_pool::cache_configuration(cache);
        let (active, _, _, paused, flashloan_enabled) = aave_config::get_flags(&config);
        assert!(active, ERESERVE_INACTIVE);
        assert!(!paused, ERESERVE_PAUSED);
        assert!(flashloan_enabled, EFLASHLOAN_DISABLED);
    }

    public fun validate_hf_and_ltv(
        user_config: &UserConfigurationMap, user: address, reserves_count: u64
    ) {
        let (_, _, _, _, health_factor, has_zero_ltv_collateral) =
            calculate_user_account_data(user_config, reserves_count, user);
        assert!(
            health_factor >= aave_config::health_factor_liquidation_threshold(),
            EHEALTH_FACTOR_TOO_LOW
        );
        assert!(!has_zero_ltv_collateral, ELTV_ZERO);
    }

    #[view]
    public fun user_account_data(user: address): (u256, u256, u256, u256, u256, bool) {
        let user_config = aave_pool::get_user_config(user);
        calculate_user_account_data(&user_config, aave_pool::reserves_count(), user)
    }

    #[view]
    public fun health_factor(user: address): u256 {
        let (_, _, _, _, hf, _) = user_account_data(user);
        hf
    }
}
