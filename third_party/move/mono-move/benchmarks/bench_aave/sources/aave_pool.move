// Ported from https://github.com/aave/aptos-aave-v3, modules
// `aave_pool::pool`, `aave_pool::pool_logic` and
// `aave_pool::default_reserve_interest_rate_strategy`.
// Copyright (c) Aave DAO and contributors.
// SPDX-License-Identifier: Apache-2.0

/// Reserve state and the interest rate model.
///
/// Every reserve is a named object derived from the pool's own object address,
/// so a caller reaches its state from the underlying's address alone with no
/// registry lookup. `ReserveData` sits in the object resource group next to
/// `ObjectCore`, so one group read answers config, indexes, rates and token
/// addresses together.
///
/// The underlying itself lives in a separate `Object<FungibleStore>` owned by
/// the pool object, which lets the pool sign its own withdrawals.
module bench::aave_pool {
    use std::bcs;
    use std::signer;
    use std::vector;
    use aptos_std::table::{Self, Table};
    use aptos_framework::fungible_asset::{Self, FungibleStore};
    use aptos_framework::object::{Self, ExtendRef, Object};
    use aptos_framework::primary_fungible_store;
    use aptos_framework::timestamp;
    use bench::aave_config::{Self, ReserveConfigurationMap, UserConfigurationMap};
    use bench::aave_math;
    use bench::aave_mock_fa;
    use bench::aave_oracle;
    use bench::aave_tokens;

    const POOL_SEED: vector<u8> = b"bench_aave_pool";
    const RESERVE_PREFIX: u8 = 0;
    const VAULT_PREFIX: u8 = 1;
    const A_TOKEN_PREFIX: u8 = 2;
    const DEBT_TOKEN_PREFIX: u8 = 3;
    const USER_PREFIX: u8 = 4;

    /// Basis points scale up to ray by 10^23, since 10000 bps is one ray.
    const BPS_TO_RAY: u256 = 100_000_000_000_000_000_000_000;

    const ENOT_ADMIN: u64 = 50;
    const EALREADY_INITIALIZED: u64 = 51;
    const ENOT_INITIALIZED: u64 = 52;
    const ERESERVE_ALREADY_ADDED: u64 = 53;
    const ERESERVE_NOT_FOUND: u64 = 54;
    const ETOO_MANY_RESERVES: u64 = 55;

    struct Pool has key {
        /// Lets the pool sign for the objects it owns, including every vault.
        extend_ref: ExtendRef,
        reserve_ids: Table<u64, address>,
        reserves_count: u64,
        user_configs: Table<address, u256>
    }

    // Aptos cannot forge a signer for a plain address outside tests, so each
    // synthetic user is an object under the pool holding its own ExtendRef.
    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct SeededUser has key {
        extend_ref: ExtendRef
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct ReserveData has key {
        configuration: ReserveConfigurationMap,
        liquidity_index: u128,
        current_liquidity_rate: u128,
        variable_borrow_index: u128,
        current_variable_borrow_rate: u128,
        last_update_timestamp: u64,
        id: u64,
        a_token: address,
        variable_debt_token: address,
        accrued_to_treasury: u256,
        /// Tracked separately from the vault balance so that a direct transfer
        /// into the vault cannot move the interest rate.
        virtual_underlying_balance: u128,
        vault: Object<FungibleStore>,
        optimal_usage_ratio: u256,
        base_variable_borrow_rate: u256,
        variable_rate_slope1: u256,
        variable_rate_slope2: u256
    }

    /// Snapshot of one reserve for the duration of a single action, so the
    /// resource group is read once and written once.
    struct ReserveCache has copy, drop {
        underlying: address,
        reserve_obj: address,
        id: u64,
        configuration: ReserveConfigurationMap,
        reserve_factor: u256,
        asset_unit: u256,
        curr_liquidity_index: u256,
        next_liquidity_index: u256,
        curr_variable_borrow_index: u256,
        next_variable_borrow_index: u256,
        curr_liquidity_rate: u256,
        curr_variable_borrow_rate: u256,
        curr_scaled_variable_debt: u256,
        next_scaled_variable_debt: u256,
        a_token: address,
        variable_debt_token: address,
        vault: Object<FungibleStore>,
        last_update_timestamp: u64
    }

    struct RateParams has copy, drop {
        optimal_usage_ratio: u256,
        base_variable_borrow_rate: u256,
        variable_rate_slope1: u256,
        variable_rate_slope2: u256
    }

    /// The subset of a reserve that the health-factor loop needs. Keeping it
    /// narrow means one group read per set bit and nothing more.
    struct ReserveAccountView has copy, drop {
        ltv: u256,
        liquidation_threshold: u256,
        asset_unit: u256,
        a_token: address,
        variable_debt_token: address,
        normalized_income: u256,
        normalized_debt: u256
    }

    public entry fun initialize(admin: &signer) {
        assert!(signer::address_of(admin) == @bench, ENOT_ADMIN);
        assert!(!exists<Pool>(@bench), EALREADY_INITIALIZED);
        let ctor = object::create_named_object(admin, POOL_SEED);
        move_to(
            admin,
            Pool {
                extend_ref: object::generate_extend_ref(&ctor),
                reserve_ids: table::new(),
                reserves_count: 0,
                user_configs: table::new()
            }
        );
        aave_oracle::initialize(admin);
    }

    public entry fun oracle_set_price(
        admin: &signer, underlying: address, price: u256
    ) {
        aave_oracle::set_price(admin, underlying, price);
    }

    public entry fun admin_add_reserve(
        admin: &signer,
        underlying: address,
        ltv: u256,
        liquidation_threshold: u256,
        liquidation_bonus: u256,
        reserve_factor: u256,
        liquidation_protocol_fee: u256,
        optimal_usage_ratio_bps: u256,
        base_variable_borrow_rate_bps: u256,
        variable_rate_slope1_bps: u256,
        variable_rate_slope2_bps: u256
    ) acquires Pool {
        assert!(signer::address_of(admin) == @bench, ENOT_ADMIN);
        assert!(exists<Pool>(@bench), ENOT_INITIALIZED);
        assert!(!exists<ReserveData>(reserve_address(underlying)), ERESERVE_ALREADY_ADDED);

        let pool = borrow_global_mut<Pool>(@bench);
        let id = pool.reserves_count;
        assert!(
            (id as u256) < aave_config::max_reserves_count(), ETOO_MANY_RESERVES
        );
        let pool_signer = object::generate_signer_for_extending(&pool.extend_ref);

        let metadata = aave_mock_fa::metadata(underlying);
        let decimals = fungible_asset::decimals(metadata);

        let vault_ctor =
            object::create_named_object(&pool_signer, seed_for(VAULT_PREFIX, underlying));
        let vault = fungible_asset::create_store(&vault_ctor, metadata);

        let a_token =
            aave_tokens::create_token(
                &pool_signer,
                seed_for(A_TOKEN_PREFIX, underlying),
                b"bench aToken",
                b"aBENCH",
                decimals,
                underlying
            );
        let variable_debt_token =
            aave_tokens::create_token(
                &pool_signer,
                seed_for(DEBT_TOKEN_PREFIX, underlying),
                b"bench variableDebtToken",
                b"vBENCH",
                decimals,
                underlying
            );

        let configuration = aave_config::init_reserve_configuration();
        aave_config::set_ltv(&mut configuration, ltv);
        aave_config::set_liquidation_threshold(&mut configuration, liquidation_threshold);
        aave_config::set_liquidation_bonus(&mut configuration, liquidation_bonus);
        aave_config::set_decimals(&mut configuration, (decimals as u256));
        aave_config::set_reserve_factor(&mut configuration, reserve_factor);
        aave_config::set_liquidation_protocol_fee(
            &mut configuration, liquidation_protocol_fee
        );
        aave_config::set_active(&mut configuration, true);
        aave_config::set_frozen(&mut configuration, false);
        aave_config::set_borrowing_enabled(&mut configuration, true);
        aave_config::set_paused(&mut configuration, false);
        aave_config::set_flashloan_enabled(&mut configuration, true);

        let reserve_ctor =
            object::create_named_object(
                &pool_signer, seed_for(RESERVE_PREFIX, underlying)
            );
        let reserve_signer = object::generate_signer(&reserve_ctor);
        move_to(
            &reserve_signer,
            ReserveData {
                configuration,
                liquidity_index: (aave_math::ray() as u128),
                current_liquidity_rate: 0,
                variable_borrow_index: (aave_math::ray() as u128),
                current_variable_borrow_rate: 0,
                last_update_timestamp: timestamp::now_microseconds(),
                id,
                a_token,
                variable_debt_token,
                accrued_to_treasury: 0,
                virtual_underlying_balance: 0,
                vault,
                optimal_usage_ratio: optimal_usage_ratio_bps * BPS_TO_RAY,
                base_variable_borrow_rate: base_variable_borrow_rate_bps * BPS_TO_RAY,
                variable_rate_slope1: variable_rate_slope1_bps * BPS_TO_RAY,
                variable_rate_slope2: variable_rate_slope2_bps * BPS_TO_RAY
            }
        );

        table::add(&mut pool.reserve_ids, id, underlying);
        pool.reserves_count = id + 1;
    }

    public fun pool_address(): address {
        object::create_object_address(&@bench, POOL_SEED)
    }

    public fun reserve_address(underlying: address): address {
        object::create_object_address(
            &pool_address(), seed_for(RESERVE_PREFIX, underlying)
        )
    }

    public fun reserve_exists(underlying: address): bool {
        exists<ReserveData>(reserve_address(underlying))
    }

    /// Where synthetic user `index` of run `seed` lives. Pure arithmetic, so a
    /// harness can compute it before the user has been created.
    public fun seeded_user_address(seed: u64, index: u64): address {
        object::create_object_address(&pool_address(), seed_for_user(seed, index))
    }

    public fun seeded_user_exists(seed: u64, index: u64): bool {
        exists<SeededUser>(seeded_user_address(seed, index))
    }

    /// Signer for synthetic user `index`, creating it on first call. Admin
    /// only, since it hands out authority over that user's balances.
    public fun seeded_user_signer(
        admin: &signer, seed: u64, index: u64
    ): signer acquires Pool, SeededUser {
        assert!(signer::address_of(admin) == @bench, ENOT_ADMIN);
        let addr = seeded_user_address(seed, index);
        if (exists<SeededUser>(addr)) {
            object::generate_signer_for_extending(
                &borrow_global<SeededUser>(addr).extend_ref
            )
        } else {
            let pool_signer =
                object::generate_signer_for_extending(
                    &borrow_global<Pool>(@bench).extend_ref
                );
            let ctor =
                object::create_named_object(&pool_signer, seed_for_user(seed, index));
            let user_signer = object::generate_signer(&ctor);
            move_to(
                &user_signer,
                SeededUser { extend_ref: object::generate_extend_ref(&ctor) }
            );
            user_signer
        }
    }

    public fun reserves_count(): u64 acquires Pool {
        borrow_global<Pool>(@bench).reserves_count
    }

    public fun reserve_address_by_id(id: u64): address acquires Pool {
        let pool = borrow_global<Pool>(@bench);
        assert!(table::contains(&pool.reserve_ids, id), ERESERVE_NOT_FOUND);
        *table::borrow(&pool.reserve_ids, id)
    }

    public fun get_user_config(user: address): UserConfigurationMap acquires Pool {
        let pool = borrow_global<Pool>(@bench);
        if (table::contains(&pool.user_configs, user)) {
            aave_config::user_configuration_from_data(
                *table::borrow(&pool.user_configs, user)
            )
        } else {
            aave_config::init_user_configuration()
        }
    }

    public fun set_user_config(
        user: address, config: &UserConfigurationMap
    ) acquires Pool {
        let pool = borrow_global_mut<Pool>(@bench);
        table::upsert(
            &mut pool.user_configs, user, aave_config::user_configuration_data(config)
        );
    }

    public fun get_configuration(underlying: address): ReserveConfigurationMap acquires ReserveData {
        borrow_global<ReserveData>(reserve_address(underlying)).configuration
    }

    public fun get_liquidity_index(underlying: address): u256 acquires ReserveData {
        (borrow_global<ReserveData>(reserve_address(underlying)).liquidity_index as u256)
    }

    public fun get_variable_borrow_index(underlying: address): u256 acquires ReserveData {
        (
            borrow_global<ReserveData>(reserve_address(underlying)).variable_borrow_index as u256
        )
    }

    public fun get_current_liquidity_rate(underlying: address): u256 acquires ReserveData {
        (
            borrow_global<ReserveData>(reserve_address(underlying)).current_liquidity_rate as u256
        )
    }

    public fun get_current_variable_borrow_rate(underlying: address): u256 acquires ReserveData {
        (
            borrow_global<ReserveData>(reserve_address(underlying)).current_variable_borrow_rate
                as u256
        )
    }

    public fun get_virtual_underlying_balance(underlying: address): u256 acquires ReserveData {
        (
            borrow_global<ReserveData>(reserve_address(underlying)).virtual_underlying_balance
                as u256
        )
    }

    public fun get_accrued_to_treasury(underlying: address): u256 acquires ReserveData {
        borrow_global<ReserveData>(reserve_address(underlying)).accrued_to_treasury
    }

    public fun get_a_token(underlying: address): address acquires ReserveData {
        borrow_global<ReserveData>(reserve_address(underlying)).a_token
    }

    public fun get_variable_debt_token(underlying: address): address acquires ReserveData {
        borrow_global<ReserveData>(reserve_address(underlying)).variable_debt_token
    }

    public fun get_reserve_id(underlying: address): u64 acquires ReserveData {
        borrow_global<ReserveData>(reserve_address(underlying)).id
    }

    /// Liquidity index brought forward to now without writing it back.
    public fun get_normalized_income(underlying: address): u256 acquires ReserveData {
        let reserve = borrow_global<ReserveData>(reserve_address(underlying));
        normalized_income_of(reserve)
    }

    public fun get_normalized_debt(underlying: address): u256 acquires ReserveData {
        let reserve = borrow_global<ReserveData>(reserve_address(underlying));
        normalized_debt_of(reserve)
    }

    public fun account_view(underlying: address): ReserveAccountView acquires ReserveData {
        let reserve = borrow_global<ReserveData>(reserve_address(underlying));
        let config = reserve.configuration;
        ReserveAccountView {
            ltv: aave_config::get_ltv(&config),
            liquidation_threshold: aave_config::get_liquidation_threshold(&config),
            asset_unit: aave_math::pow(10, aave_config::get_decimals(&config)),
            a_token: reserve.a_token,
            variable_debt_token: reserve.variable_debt_token,
            normalized_income: normalized_income_of(reserve),
            normalized_debt: normalized_debt_of(reserve)
        }
    }

    public fun view_ltv(self: &ReserveAccountView): u256 {
        self.ltv
    }

    public fun view_liquidation_threshold(self: &ReserveAccountView): u256 {
        self.liquidation_threshold
    }

    public fun view_asset_unit(self: &ReserveAccountView): u256 {
        self.asset_unit
    }

    public fun view_a_token(self: &ReserveAccountView): address {
        self.a_token
    }

    public fun view_variable_debt_token(self: &ReserveAccountView): address {
        self.variable_debt_token
    }

    public fun view_normalized_income(self: &ReserveAccountView): u256 {
        self.normalized_income
    }

    public fun view_normalized_debt(self: &ReserveAccountView): u256 {
        self.normalized_debt
    }

    public fun cache(underlying: address): ReserveCache acquires ReserveData {
        let reserve_obj = reserve_address(underlying);
        assert!(exists<ReserveData>(reserve_obj), ERESERVE_NOT_FOUND);
        let reserve = borrow_global<ReserveData>(reserve_obj);
        let config = reserve.configuration;
        let curr_scaled_variable_debt =
            aave_tokens::scaled_total_supply(reserve.variable_debt_token);
        ReserveCache {
            underlying,
            reserve_obj,
            id: reserve.id,
            configuration: config,
            reserve_factor: aave_config::get_reserve_factor(&config),
            asset_unit: aave_math::pow(10, aave_config::get_decimals(&config)),
            curr_liquidity_index: (reserve.liquidity_index as u256),
            next_liquidity_index: (reserve.liquidity_index as u256),
            curr_variable_borrow_index: (reserve.variable_borrow_index as u256),
            next_variable_borrow_index: (reserve.variable_borrow_index as u256),
            curr_liquidity_rate: (reserve.current_liquidity_rate as u256),
            curr_variable_borrow_rate: (reserve.current_variable_borrow_rate as u256),
            curr_scaled_variable_debt,
            next_scaled_variable_debt: curr_scaled_variable_debt,
            a_token: reserve.a_token,
            variable_debt_token: reserve.variable_debt_token,
            vault: reserve.vault,
            last_update_timestamp: reserve.last_update_timestamp
        }
    }

    public fun cache_underlying(self: &ReserveCache): address {
        self.underlying
    }

    public fun cache_id(self: &ReserveCache): u64 {
        self.id
    }

    public fun cache_configuration(self: &ReserveCache): ReserveConfigurationMap {
        self.configuration
    }

    public fun cache_reserve_factor(self: &ReserveCache): u256 {
        self.reserve_factor
    }

    public fun cache_asset_unit(self: &ReserveCache): u256 {
        self.asset_unit
    }

    public fun cache_curr_liquidity_index(self: &ReserveCache): u256 {
        self.curr_liquidity_index
    }

    public fun cache_next_liquidity_index(self: &ReserveCache): u256 {
        self.next_liquidity_index
    }

    public fun cache_curr_variable_borrow_index(self: &ReserveCache): u256 {
        self.curr_variable_borrow_index
    }

    public fun cache_next_variable_borrow_index(self: &ReserveCache): u256 {
        self.next_variable_borrow_index
    }

    public fun cache_curr_scaled_variable_debt(self: &ReserveCache): u256 {
        self.curr_scaled_variable_debt
    }

    public fun cache_next_scaled_variable_debt(self: &ReserveCache): u256 {
        self.next_scaled_variable_debt
    }

    public fun cache_set_next_scaled_variable_debt(
        self: &mut ReserveCache, value: u256
    ) {
        self.next_scaled_variable_debt = value;
    }

    public fun cache_a_token(self: &ReserveCache): address {
        self.a_token
    }

    public fun cache_variable_debt_token(self: &ReserveCache): address {
        self.variable_debt_token
    }

    public fun cache_last_update_timestamp(self: &ReserveCache): u64 {
        self.last_update_timestamp
    }

    /// Rolls both indexes forward to now and persists them. Repeated calls at
    /// the same timestamp are free, which matters because every action starts
    /// here.
    public fun update_state(cache: &mut ReserveCache) acquires ReserveData {
        let now = timestamp::now_microseconds();
        if (cache.last_update_timestamp == now) {
            return
        };
        if (cache.curr_liquidity_rate != 0) {
            let cumulated =
                aave_math::calculate_linear_interest(
                    cache.curr_liquidity_rate, cache.last_update_timestamp
                );
            cache.next_liquidity_index = aave_math::ray_mul(
                cumulated, cache.curr_liquidity_index
            );
        };
        if (cache.curr_scaled_variable_debt != 0) {
            let cumulated =
                aave_math::calculate_compounded_interest_now(
                    cache.curr_variable_borrow_rate, cache.last_update_timestamp
                );
            cache.next_variable_borrow_index = aave_math::ray_mul(
                cumulated, cache.curr_variable_borrow_index
            );
        };

        let treasury_delta = accrued_treasury_delta(cache);
        cache.last_update_timestamp = now;

        let reserve = borrow_global_mut<ReserveData>(cache.reserve_obj);
        reserve.liquidity_index = (cache.next_liquidity_index as u128);
        reserve.variable_borrow_index = (cache.next_variable_borrow_index as u128);
        reserve.accrued_to_treasury = reserve.accrued_to_treasury + treasury_delta;
        reserve.last_update_timestamp = now;
    }

    /// Recomputes both rates from the post-action liquidity and stores the new
    /// virtual balance.
    public fun update_interest_rates_and_virtual_balance(
        cache: &ReserveCache, liquidity_added: u256, liquidity_taken: u256
    ) acquires ReserveData {
        let total_variable_debt =
            aave_math::ray_mul_up(
                cache.next_scaled_variable_debt, cache.next_variable_borrow_index
            );
        let reserve = borrow_global_mut<ReserveData>(cache.reserve_obj);
        let virtual_balance = (reserve.virtual_underlying_balance as u256);
        let params = RateParams {
            optimal_usage_ratio: reserve.optimal_usage_ratio,
            base_variable_borrow_rate: reserve.base_variable_borrow_rate,
            variable_rate_slope1: reserve.variable_rate_slope1,
            variable_rate_slope2: reserve.variable_rate_slope2
        };
        let (next_liquidity_rate, next_variable_rate) =
            calculate_interest_rates(
                &params,
                liquidity_added,
                liquidity_taken,
                total_variable_debt,
                cache.reserve_factor,
                virtual_balance
            );

        reserve.current_liquidity_rate = (next_liquidity_rate as u128);
        reserve.current_variable_borrow_rate = (next_variable_rate as u128);
        reserve.virtual_underlying_balance =
            ((virtual_balance + liquidity_added - liquidity_taken) as u128);
    }

    /// Folds `amount` of extra liquidity into the index instead of minting,
    /// which is how a flash loan premium reaches every supplier at once.
    public fun cumulate_to_liquidity_index(
        cache: &mut ReserveCache, total_liquidity: u256, amount: u256
    ): u256 acquires ReserveData {
        let result =
            aave_math::ray_mul(
                aave_math::ray_div(
                    aave_math::wad_to_ray(amount), aave_math::wad_to_ray(total_liquidity)
                ) + aave_math::ray(),
                cache.next_liquidity_index
            );
        cache.next_liquidity_index = result;
        let reserve = borrow_global_mut<ReserveData>(cache.reserve_obj);
        reserve.liquidity_index = (result as u128);
        result
    }

    public fun vault_deposit(
        from: &signer, cache: &ReserveCache, amount: u64
    ) {
        let fa =
            primary_fungible_store::withdraw(
                from, aave_mock_fa::metadata(cache.underlying), amount
            );
        fungible_asset::deposit(cache.vault, fa);
    }

    public fun vault_withdraw(
        cache: &ReserveCache, to: address, amount: u64
    ) acquires Pool {
        let pool_signer =
            object::generate_signer_for_extending(
                &borrow_global<Pool>(@bench).extend_ref
            );
        let fa = fungible_asset::withdraw(&pool_signer, cache.vault, amount);
        primary_fungible_store::deposit(to, fa);
    }

    public fun vault_balance(underlying: address): u64 acquires ReserveData {
        let reserve = borrow_global<ReserveData>(reserve_address(underlying));
        fungible_asset::balance(reserve.vault)
    }

    public fun asset_price(underlying: address): u256 {
        aave_oracle::get_price(underlying)
    }

    fun normalized_income_of(reserve: &ReserveData): u256 {
        if (reserve.last_update_timestamp == timestamp::now_microseconds()) {
            (reserve.liquidity_index as u256)
        } else {
            aave_math::ray_mul(
                aave_math::calculate_linear_interest(
                    (reserve.current_liquidity_rate as u256),
                    reserve.last_update_timestamp
                ),
                (reserve.liquidity_index as u256)
            )
        }
    }

    fun normalized_debt_of(reserve: &ReserveData): u256 {
        if (reserve.last_update_timestamp == timestamp::now_microseconds()) {
            (reserve.variable_borrow_index as u256)
        } else {
            aave_math::ray_mul(
                aave_math::calculate_compounded_interest_now(
                    (reserve.current_variable_borrow_rate as u256),
                    reserve.last_update_timestamp
                ),
                (reserve.variable_borrow_index as u256)
            )
        }
    }

    /// The share of freshly accrued borrow interest that the reserve factor
    /// keeps, expressed in scaled aToken units.
    fun accrued_treasury_delta(cache: &ReserveCache): u256 {
        if (cache.reserve_factor == 0) {
            return 0
        };
        let prev_total_debt =
            aave_math::ray_mul(
                cache.curr_scaled_variable_debt, cache.curr_variable_borrow_index
            );
        let curr_total_debt =
            aave_math::ray_mul(
                cache.curr_scaled_variable_debt, cache.next_variable_borrow_index
            );
        let amount_to_mint =
            aave_math::percent_mul(
                curr_total_debt - prev_total_debt, cache.reserve_factor
            );
        if (amount_to_mint == 0) {
            return 0
        };
        aave_math::ray_div_down(amount_to_mint, cache.next_liquidity_index)
    }

    /// Two-slope model: the borrow rate rises gently up to the optimal usage
    /// ratio and steeply past it, which is what pushes utilisation back down.
    fun calculate_interest_rates(
        params: &RateParams,
        liquidity_added: u256,
        liquidity_taken: u256,
        total_debt: u256,
        reserve_factor: u256,
        virtual_underlying_balance: u256
    ): (u256, u256) {
        let variable_borrow_rate = params.base_variable_borrow_rate;
        if (total_debt == 0) {
            return (0, variable_borrow_rate)
        };
        let available_liquidity =
            virtual_underlying_balance + liquidity_added - liquidity_taken;
        let available_plus_debt = available_liquidity + total_debt;
        let borrow_usage_ratio = aave_math::ray_div(total_debt, available_plus_debt);
        let supply_usage_ratio = borrow_usage_ratio;

        if (borrow_usage_ratio > params.optimal_usage_ratio) {
            let excess =
                aave_math::ray_div(
                    borrow_usage_ratio - params.optimal_usage_ratio,
                    aave_math::ray() - params.optimal_usage_ratio
                );
            variable_borrow_rate = variable_borrow_rate + params.variable_rate_slope1
                + aave_math::ray_mul(params.variable_rate_slope2, excess);
        } else {
            variable_borrow_rate = variable_borrow_rate
                + aave_math::ray_div(
                    aave_math::ray_mul(
                        params.variable_rate_slope1, borrow_usage_ratio
                    ),
                    params.optimal_usage_ratio
                );
        };

        let liquidity_rate =
            aave_math::percent_mul(
                aave_math::ray_mul(variable_borrow_rate, supply_usage_ratio),
                aave_math::percentage_factor() - reserve_factor
            );
        (liquidity_rate, variable_borrow_rate)
    }

    /// Prefixing keeps the four objects derived from one underlying distinct.
    fun seed_for(prefix: u8, underlying: address): vector<u8> {
        let seed = vector[prefix];
        vector::append(&mut seed, bcs::to_bytes(&underlying));
        seed
    }

    fun seed_for_user(seed: u64, index: u64): vector<u8> {
        let bytes = vector[USER_PREFIX];
        vector::append(&mut bytes, bcs::to_bytes(&seed));
        vector::append(&mut bytes, bcs::to_bytes(&index));
        bytes
    }
}
