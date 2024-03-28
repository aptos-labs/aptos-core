module SwapDeployer::AnimeSwapPoolV1 {
    use ResourceAccountDeployer::LPCoinV1::LPCoin;
    use SwapDeployer::AnimeSwapPoolV1Library;
    use SwapDeployer::LPResourceAccount;
    use std::signer;
    use std::type_info::{Self, TypeInfo};
    use std::string::utf8;
    use std::event;
    use std::vector;
    use aptos_framework::timestamp;
    use aptos_framework::coin::{Self, Coin, MintCapability, FreezeCapability, BurnCapability};
    use aptos_framework::account::{Self, SignerCapability};
    use u256::u256;
    use uq64x64::uq64x64;
    // use std::debug;    // For debug

    /// pool data
    struct LiquidityPool<phantom X, phantom Y> has key {
        coin_x_reserve: Coin<X>,
        coin_y_reserve: Coin<Y>,
        last_block_timestamp: u64,
        last_price_x_cumulative: u128,
        last_price_y_cumulative: u128,
        k_last: u128,
        lp_mint_cap: MintCapability<LPCoin<X, Y>>,
        lp_freeze_cap: FreezeCapability<LPCoin<X, Y>>,
        lp_burn_cap: BurnCapability<LPCoin<X, Y>>,
        locked: bool,
    }

    /// global config data
    struct AdminData has key {
        signer_cap: SignerCapability,
        dao_fee_to: address,
        admin_address: address,
        dao_fee: u8,   // 1/(dao_fee+1) comes to dao_fee_to if dao_fee_on
        swap_fee: u64,  // BP, swap_fee * 1/10000
        dao_fee_on: bool,   // default: true
        is_pause: bool, // pause swap
    }

    struct PairMeta has drop, store, copy {
        coin_x: TypeInfo,
        coin_y: TypeInfo,
        lp_coin: TypeInfo,
    }

    /// pair list
    struct PairInfo has key {
        pair_list: vector<PairMeta>,
    }

    struct Events<phantom X, phantom Y> has key {
        pair_created_event: event::EventHandle<PairCreatedEvent<X, Y>>,
        mint_event: event::EventHandle<MintEvent<X, Y>>,
        burn_event: event::EventHandle<BurnEvent<X, Y>>,
        swap_event: event::EventHandle<SwapEvent<X, Y>>,
        sync_event: event::EventHandle<SyncEvent<X, Y>>,
        flash_swap_event: event::EventHandle<FlashSwapEvent<X, Y>>,
    }

    struct PairCreatedEvent<phantom X, phantom Y> has drop, store {
        meta: PairMeta,
    }

    struct MintEvent<phantom X, phantom Y> has drop, store {
        amount_x: u64,
        amount_y: u64,
        liquidity: u64,
    }

    struct BurnEvent<phantom X, phantom Y> has drop, store {
        amount_x: u64,
        amount_y: u64,
        liquidity: u64,
    }

    struct SwapEvent<phantom X, phantom Y> has drop, store {
        amount_x_in: u64,
        amount_y_in: u64,
        amount_x_out: u64,
        amount_y_out: u64,
    }

    struct SyncEvent<phantom X, phantom Y> has drop, store {
        reserve_x: u64,
        reserve_y: u64,
        last_price_x_cumulative: u128,
        last_price_y_cumulative: u128,
    }

    struct FlashSwapEvent<phantom X, phantom Y> has drop, store {
        loan_coin_x: u64,
        loan_coin_y: u64,
        repay_coin_x: u64,
        repay_coin_y: u64,
    }

    /// no copy, no drop
    struct FlashSwap<phantom X, phantom Y> {
        loan_coin_x: u64,
        loan_coin_y: u64
    }

    const MINIMUM_LIQUIDITY: u64 = 1000;
    const MAX_U64: u64 = 18446744073709551615u64;

    /// When contract error
    const ERR_INTERNAL_ERROR: u64 = 102;
    /// When user is not admin
    const ERR_FORBIDDEN: u64 = 103;
    /// When not enough amount for pool
    const ERR_INSUFFICIENT_AMOUNT: u64 = 104;
    /// When not enough liquidity amount
    const ERR_INSUFFICIENT_LIQUIDITY: u64 = 105;
    /// When not enough liquidity minted
    const ERR_INSUFFICIENT_LIQUIDITY_MINT: u64 = 106;
    /// When not enough liquidity burned
    const ERR_INSUFFICIENT_LIQUIDITY_BURN: u64 = 107;
    /// When not enough X amount
    const ERR_INSUFFICIENT_X_AMOUNT: u64 = 108;
    /// When not enough Y amount
    const ERR_INSUFFICIENT_Y_AMOUNT: u64 = 109;
    /// When not enough input amount
    const ERR_INSUFFICIENT_INPUT_AMOUNT: u64 = 110;
    /// When not enough output amount
    const ERR_INSUFFICIENT_OUTPUT_AMOUNT: u64 = 111;
    /// When contract K error
    const ERR_K_ERROR: u64 = 112;
    /// When already exists on account
    const ERR_PAIR_ALREADY_EXIST: u64 = 115;
    /// When not exists on account
    const ERR_PAIR_NOT_EXIST: u64 = 116;
    /// When error loan amount
    const ERR_LOAN_ERROR: u64 = 117;
    /// When contract is not reentrant
    const ERR_LOCK_ERROR: u64 = 118;
    /// When pair has wrong ordering
    const ERR_PAIR_ORDER_ERROR: u64 = 119;
    /// When contract is paused
    const ERR_PAUSABLE_ERROR: u64 = 120;

    const DEPLOYER_ADDRESS: address = @SwapDeployer;
    const RESOURCE_ACCOUNT_ADDRESS: address = @ResourceAccountDeployer;

    // initialize
    fun init_module(admin: &signer) {
        // init admin data
        let signer_cap = LPResourceAccount::retrieve_signer_cap(admin);
        let resource_account = &account::create_signer_with_capability(&signer_cap);
        move_to(resource_account, AdminData {
            signer_cap,
            dao_fee_to: DEPLOYER_ADDRESS,
            admin_address: DEPLOYER_ADDRESS,
            dao_fee: 5,         // 1/6 to dao fee
            swap_fee: 30,       // 0.3%
            dao_fee_on: true,  // default true
            is_pause: false,    // default false
        });
        // init pair info
        move_to(resource_account, PairInfo{
            pair_list: vector::empty(),
        });
    }

    /**
     *  Helper functions, for internal use. Some are public for helping other contracts calculation
     */

    /// get reserves size
    /// always return (X_reserve, Y_reserve)
    public fun get_reserves_size<X, Y>(): (u64, u64) acquires LiquidityPool {
        if (AnimeSwapPoolV1Library::compare<X, Y>()) {
            let lp = borrow_global<LiquidityPool<X, Y>>(RESOURCE_ACCOUNT_ADDRESS);
            (coin::value(&lp.coin_x_reserve), coin::value(&lp.coin_y_reserve))
        } else {
            let lp = borrow_global<LiquidityPool<Y, X>>(RESOURCE_ACCOUNT_ADDRESS);
            (coin::value(&lp.coin_y_reserve), coin::value(&lp.coin_x_reserve))
        }
    }

    /// get amounts out, 1 pair
    public fun get_amounts_out_1_pair<X, Y>(
        amount_in: u64
    ): u64 acquires LiquidityPool, AdminData {
        let swap_fee = borrow_global<AdminData>(RESOURCE_ACCOUNT_ADDRESS).swap_fee;
        let (reserve_in, reserve_out) = get_reserves_size<X, Y>();
        let amount_out = AnimeSwapPoolV1Library::get_amount_out(amount_in, reserve_in, reserve_out, swap_fee);
        amount_out
    }

    /// get amounts out, 2 pairs
    public fun get_amounts_out_2_pair<X, Y, Z>(
        amount_in: u64
    ): u64 acquires LiquidityPool, AdminData {
        let swap_fee = borrow_global<AdminData>(RESOURCE_ACCOUNT_ADDRESS).swap_fee;
        let (reserve_in, reserve_out) = get_reserves_size<X, Y>();
        let amount_mid = AnimeSwapPoolV1Library::get_amount_out(amount_in, reserve_in, reserve_out, swap_fee);
        (reserve_in, reserve_out) = get_reserves_size<Y, Z>();
        let amount_out = AnimeSwapPoolV1Library::get_amount_out(amount_mid, reserve_in, reserve_out, swap_fee);
        amount_out
    }

    /// get amounts out, 3 pairs
    public fun get_amounts_out_3_pair<X, Y, Z, W>(
        amount_in: u64
    ): u64 acquires LiquidityPool, AdminData {
        let swap_fee = borrow_global<AdminData>(RESOURCE_ACCOUNT_ADDRESS).swap_fee;
        let (reserve_in, reserve_out) = get_reserves_size<X, Y>();
        let amount_mid = AnimeSwapPoolV1Library::get_amount_out(amount_in, reserve_in, reserve_out, swap_fee);
        (reserve_in, reserve_out) = get_reserves_size<Y, Z>();
        let amount_mid = AnimeSwapPoolV1Library::get_amount_out(amount_mid, reserve_in, reserve_out, swap_fee);
        (reserve_in, reserve_out) = get_reserves_size<Z, W>();
        let amount_out = AnimeSwapPoolV1Library::get_amount_out(amount_mid, reserve_in, reserve_out, swap_fee);
        amount_out
    }

    /// get amounts in, 1 pair
    public fun get_amounts_in_1_pair<X, Y>(
        amount_out: u64
    ): u64 acquires LiquidityPool, AdminData {
        let swap_fee = borrow_global<AdminData>(RESOURCE_ACCOUNT_ADDRESS).swap_fee;
        let (reserve_in, reserve_out) = get_reserves_size<X, Y>();
        let amount_in = AnimeSwapPoolV1Library::get_amount_in(amount_out, reserve_in, reserve_out, swap_fee);
        amount_in
    }

    /// get amounts in, 2 pairs
    public fun get_amounts_in_2_pair<X, Y, Z>(
        amount_out: u64
    ): u64 acquires LiquidityPool, AdminData {
        let swap_fee = borrow_global<AdminData>(RESOURCE_ACCOUNT_ADDRESS).swap_fee;
        let (reserve_in, reserve_out) = get_reserves_size<Y, Z>();
        let amount_mid = AnimeSwapPoolV1Library::get_amount_in(amount_out, reserve_in, reserve_out, swap_fee);
        (reserve_in, reserve_out) = get_reserves_size<X, Y>();
        let amount_in = AnimeSwapPoolV1Library::get_amount_in(amount_mid, reserve_in, reserve_out, swap_fee);
        amount_in
    }

    /// get amounts in, 3 pairs
    public fun get_amounts_in_3_pair<X, Y, Z, W>(
        amount_out: u64
    ): u64 acquires LiquidityPool, AdminData {
        let swap_fee = borrow_global<AdminData>(RESOURCE_ACCOUNT_ADDRESS).swap_fee;
        let (reserve_in, reserve_out) = get_reserves_size<Z, W>();
        let amount_mid = AnimeSwapPoolV1Library::get_amount_in(amount_out, reserve_in, reserve_out, swap_fee);
        (reserve_in, reserve_out) = get_reserves_size<Y, Z>();
        let amount_mid = AnimeSwapPoolV1Library::get_amount_in(amount_mid, reserve_in, reserve_out, swap_fee);
        (reserve_in, reserve_out) = get_reserves_size<X, Y>();
        let amount_in = AnimeSwapPoolV1Library::get_amount_in(amount_mid, reserve_in, reserve_out, swap_fee);
        amount_in
    }

    /// get pair meta with `X`, `Y`
    public fun get_pair_meta<X, Y>(): PairMeta {
        let coin_x_type_info = type_info::type_of<X>();
        let coin_y_type_info = type_info::type_of<Y>();
        let lp_coin_type_info = type_info::type_of<LPCoin<X, Y>>();
        PairMeta {
            coin_x: coin_x_type_info,
            coin_y: coin_y_type_info,
            lp_coin: lp_coin_type_info,
        }
    }

    /// assert lp unlocked
    fun assert_lp_unlocked<X, Y>() acquires LiquidityPool {
        assert!(exists<LiquidityPool<X, Y>>(RESOURCE_ACCOUNT_ADDRESS), ERR_PAIR_NOT_EXIST);
        let lp = borrow_global<LiquidityPool<X, Y>>(RESOURCE_ACCOUNT_ADDRESS);
        assert!(lp.locked == false, ERR_LOCK_ERROR);
    }

    /// assert swap paused
    fun assert_paused() acquires AdminData {
        assert!(borrow_global<AdminData>(RESOURCE_ACCOUNT_ADDRESS).is_pause, ERR_PAUSABLE_ERROR);
    }

    /// assert swap not paused
    fun assert_not_paused() acquires AdminData {
        assert!(!borrow_global<AdminData>(RESOURCE_ACCOUNT_ADDRESS).is_pause, ERR_PAUSABLE_ERROR);
    }

    /// return pair admin account signer
    fun get_resource_account_signer(): signer acquires AdminData {
        let signer_cap = &borrow_global<AdminData>(RESOURCE_ACCOUNT_ADDRESS).signer_cap;
        account::create_signer_with_capability(signer_cap)
    }

    /// Calculate optimal amounts of coins to add
    public fun calc_optimal_coin_values<X, Y>(
        amount_x_desired: u64,
        amount_y_desired: u64,
        amount_x_min: u64,
        amount_y_min: u64
    ): (u64, u64) acquires LiquidityPool {
        let lp = borrow_global<LiquidityPool<X, Y>>(RESOURCE_ACCOUNT_ADDRESS);
        let (reserve_x, reserve_y) = (coin::value(&lp.coin_x_reserve), coin::value(&lp.coin_y_reserve));
        if (reserve_x == 0 && reserve_y == 0) {
            (amount_x_desired, amount_y_desired)
        } else {
            let amount_y_optimal = AnimeSwapPoolV1Library::quote(amount_x_desired, reserve_x, reserve_y);
            if (amount_y_optimal <= amount_y_desired) {
                assert!(amount_y_optimal >= amount_y_min, ERR_INSUFFICIENT_Y_AMOUNT);
                (amount_x_desired, amount_y_optimal)
            } else {
                let amount_x_optimal = AnimeSwapPoolV1Library::quote(amount_y_desired, reserve_y, reserve_x);
                assert!(amount_x_optimal <= amount_x_desired, ERR_INTERNAL_ERROR);
                assert!(amount_x_optimal >= amount_x_min, ERR_INSUFFICIENT_X_AMOUNT);
                (amount_x_optimal, amount_y_desired)
            }
        }
    }

    /// k should not decrease
    fun assert_k_increase(
        balance_x: u64,
        balance_y: u64,
        amount_x_in: u64,
        amount_y_in: u64,
        reserve_x: u64,
        reserve_y: u64,
    ) acquires AdminData {
        let swap_fee = borrow_global<AdminData>(RESOURCE_ACCOUNT_ADDRESS).swap_fee;
        let balance_x_adjusted = (balance_x as u128) * 10000 - (amount_x_in as u128) * (swap_fee as u128);
        let balance_y_adjusted = (balance_y as u128) * 10000 - (amount_y_in as u128) * (swap_fee as u128);
        let balance_xy_old_not_scaled = (reserve_x as u128) * (reserve_y as u128);
        let scale = 100000000;
        // should be: new_reserve_x * new_reserve_y > old_reserve_x * old_eserve_y
        // gas saving
        if (
            AnimeSwapPoolV1Library::is_overflow_mul(balance_x_adjusted, balance_y_adjusted)
            || AnimeSwapPoolV1Library::is_overflow_mul(balance_xy_old_not_scaled, scale)
        ) {
            let balance_xy_adjusted = u256::mul(u256::from_u128(balance_x_adjusted), u256::from_u128(balance_y_adjusted));
            let balance_xy_old = u256::mul(u256::from_u128(balance_xy_old_not_scaled), u256::from_u128(scale));
            assert!(u256::compare(&balance_xy_adjusted, &balance_xy_old) == 2, ERR_K_ERROR);
        } else {
            assert!(balance_x_adjusted * balance_y_adjusted >= balance_xy_old_not_scaled * scale, ERR_K_ERROR)
        };
    }

    /// update cumulative, coin_reserve, block_timestamp
    fun update_internal<X, Y>(
        lp: &mut LiquidityPool<X, Y>,
        balance_x: u64, // new reserve value
        balance_y: u64,
        reserve_x: u64, // old reserve value
        reserve_y: u64
    ) acquires Events {
        let now = timestamp::now_seconds();
        let time_elapsed = ((now - lp.last_block_timestamp) as u128);
        if (time_elapsed > 0 && reserve_x != 0 && reserve_y != 0) {
            // allow overflow u128
            let last_price_x_cumulative_delta = uq64x64::to_u128(uq64x64::fraction(reserve_y, reserve_x)) * time_elapsed;
            lp.last_price_x_cumulative = AnimeSwapPoolV1Library::overflow_add(lp.last_price_x_cumulative, last_price_x_cumulative_delta);

            let last_price_y_cumulative_delta = uq64x64::to_u128(uq64x64::fraction(reserve_x, reserve_y)) * time_elapsed;
            lp.last_price_y_cumulative = AnimeSwapPoolV1Library::overflow_add(lp.last_price_y_cumulative, last_price_y_cumulative_delta);
        };
        lp.last_block_timestamp = now;
        // event
        let events = borrow_global_mut<Events<X, Y>>(RESOURCE_ACCOUNT_ADDRESS);
        event::emit_event(&mut events.sync_event, SyncEvent {
            reserve_x: balance_x,
            reserve_y: balance_y,
            last_price_x_cumulative: lp.last_price_x_cumulative,
            last_price_y_cumulative: lp.last_price_y_cumulative,
        });
    }

    fun mint_fee_interval<X, Y>(
        lp: &mut LiquidityPool<X, Y>,
        admin_data: &AdminData
    ): bool {
        let fee_on = admin_data.dao_fee_on;
        let k_last = lp.k_last;
        if (fee_on) {
            if (k_last != 0) {
                let reserve_x = coin::value(&lp.coin_x_reserve);
                let reserve_y = coin::value(&lp.coin_y_reserve);
                let root_k = AnimeSwapPoolV1Library::sqrt(reserve_x, reserve_y);
                let root_k_last = AnimeSwapPoolV1Library::sqrt_128(k_last);
                let total_supply = AnimeSwapPoolV1Library::get_lpcoin_total_supply<LPCoin<X, Y>>();
                if (root_k > root_k_last) {
                    let delta_k = ((root_k - root_k_last) as u128);
                    // gas saving
                    if (AnimeSwapPoolV1Library::is_overflow_mul(total_supply, delta_k)) {
                        let numerator = u256::mul(u256::from_u128(total_supply), u256::from_u128(delta_k));
                        let denominator = u256::from_u128((root_k as u128) * (admin_data.dao_fee as u128) + (root_k_last as u128));
                        let liquidity = u256::as_u64(u256::div(numerator, denominator));
                        if (liquidity > 0) {
                            mint_coin<X, Y>(&account::create_signer_with_capability(&admin_data.signer_cap), liquidity, &lp.lp_mint_cap);
                        };
                    } else {
                        let numerator = total_supply * delta_k;
                        let denominator = (root_k as u128) * (admin_data.dao_fee as u128) + (root_k_last as u128);
                        let liquidity = ((numerator / denominator) as u64);
                        if (liquidity > 0) {
                            mint_coin<X, Y>(&account::create_signer_with_capability(&admin_data.signer_cap), liquidity, &lp.lp_mint_cap);
                        };
                    };
                }
            }
        } else if (k_last != 0) {
            lp.k_last = 0;
        };
        fee_on
    }

    /// mint coin with MintCapability
    fun mint_coin<X, Y>(
        account: &signer,
        amount: u64,
        mint_cap: &MintCapability<LPCoin<X, Y>>
    ) {
        let acc_addr = signer::address_of(account);
        if (!coin::is_account_registered<LPCoin<X, Y>>(acc_addr)) {
            coin::register<LPCoin<X, Y>>(account);
        };
        let coins = coin::mint<LPCoin<X, Y>>(amount, mint_cap);
        coin::deposit(acc_addr, coins);
    }

    /**
     * Entry functions
     */

    /// Add liquidity. If pair not exist, create pair first
    /// No require for pair order sorting
    public entry fun add_liquidity_entry<X, Y>(
        account: &signer,
        amount_x_desired: u64,
        amount_y_desired: u64,
        amount_x_min: u64,
        amount_y_min: u64,
    ) acquires LiquidityPool, AdminData, PairInfo, Events {
        if (AnimeSwapPoolV1Library::compare<X, Y>()) {
            if (!exists<LiquidityPool<X, Y>>(RESOURCE_ACCOUNT_ADDRESS)) {
                create_pair<X, Y>();
            };
            add_liquidity<X, Y>(account, amount_x_desired, amount_y_desired, amount_x_min, amount_y_min);
        } else {
            if (!exists<LiquidityPool<Y, X>>(RESOURCE_ACCOUNT_ADDRESS)) {
                create_pair<Y, X>();
            };
            add_liquidity<Y, X>(account, amount_y_desired, amount_x_desired, amount_y_min, amount_x_min);
        }
    }

    /// Remove liquidity
    /// No require for pair order sorting
    public entry fun remove_liquidity_entry<X, Y>(
        account: &signer,
        liquidity: u64,
        amount_x_min: u64,
        amount_y_min: u64,
    ) acquires LiquidityPool, AdminData, Events {
        let (x_out, y_out);
        if (AnimeSwapPoolV1Library::compare<X, Y>()) {
            let coins = coin::withdraw<LPCoin<X, Y>>(account, liquidity);
            (x_out, y_out) = remove_liquidity<X, Y>(coins, amount_x_min, amount_y_min);
        } else {
            let coins = coin::withdraw<LPCoin<Y, X>>(account, liquidity);
            (y_out, x_out) = remove_liquidity<Y, X>(coins, amount_y_min, amount_x_min);
        };
        // transfer
        let account_addr = signer::address_of(account);
        coin::deposit(account_addr, x_out);
        coin::deposit(account_addr, y_out);
    }

    /// 1 pair swap X->Y
    public entry fun swap_exact_coins_for_coins_entry<X, Y>(
        account: &signer,
        amount_in: u64,
        amount_out_min: u64,
    ) acquires LiquidityPool, AdminData, Events {
        // swap
        let coins_in = coin::withdraw<X>(account, amount_in);
        let coins_out;
        coins_out = swap_coins_for_coins<X, Y>(coins_in);
        assert!(coin::value(&coins_out) >= amount_out_min, ERR_INSUFFICIENT_OUTPUT_AMOUNT);
        AnimeSwapPoolV1Library::register_coin<Y>(account);
        coin::deposit<Y>(signer::address_of(account), coins_out);
    }

    /// 2 pairs swap X->Y->Z
    public entry fun swap_exact_coins_for_coins_2_pair_entry<X, Y, Z>(
        account: &signer,
        amount_in: u64,
        amount_out_min: u64,
    ) acquires LiquidityPool, AdminData, Events {
        // swap
        let coins_in = coin::withdraw<X>(account, amount_in);
        let coins_out;
        let coins_mid;
        coins_mid = swap_coins_for_coins<X, Y>(coins_in);
        coins_out = swap_coins_for_coins<Y, Z>(coins_mid);
        assert!(coin::value(&coins_out) >= amount_out_min, ERR_INSUFFICIENT_OUTPUT_AMOUNT);
        AnimeSwapPoolV1Library::register_coin<Z>(account);
        coin::deposit<Z>(signer::address_of(account), coins_out);
    }

    /// 3 pairs swap X->Y->Z->W
    public entry fun swap_exact_coins_for_coins_3_pair_entry<X, Y, Z, W>(
        account: &signer,
        amount_in: u64,
        amount_out_min: u64,
    ) acquires LiquidityPool, AdminData, Events {
        // swap
        let coins_in = coin::withdraw<X>(account, amount_in);
        let coins_out;
        let coins_mid;
        let coins_mid_2;
        coins_mid = swap_coins_for_coins<X, Y>(coins_in);
        coins_mid_2 = swap_coins_for_coins<Y, Z>(coins_mid);
        coins_out = swap_coins_for_coins<Z, W>(coins_mid_2);
        assert!(coin::value(&coins_out) >= amount_out_min, ERR_INSUFFICIENT_OUTPUT_AMOUNT);
        AnimeSwapPoolV1Library::register_coin<W>(account);
        coin::deposit<W>(signer::address_of(account), coins_out);
    }

    /// 1 pair swap X->Y
    public entry fun swap_coins_for_exact_coins_entry<X, Y>(
        account: &signer,
        amount_out: u64,
        amount_in_max: u64,
    ) acquires LiquidityPool, AdminData, Events {
        let amount_in = get_amounts_in_1_pair<X, Y>(amount_out);
        assert!(amount_in <= amount_in_max, ERR_INSUFFICIENT_INPUT_AMOUNT);
        let coins_in = coin::withdraw<X>(account, amount_in);
        let coins_out;
        coins_out = swap_coins_for_coins<X, Y>(coins_in);
        AnimeSwapPoolV1Library::register_coin<Y>(account);
        coin::deposit<Y>(signer::address_of(account), coins_out);
    }

    /// 2 pairs swap X->Y->Z
    public entry fun swap_coins_for_exact_coins_2_pair_entry<X, Y, Z>(
        account: &signer,
        amount_out: u64,
        amount_in_max: u64,
    ) acquires LiquidityPool, AdminData, Events {
        let amount_in = get_amounts_in_2_pair<X, Y, Z>(amount_out);
        assert!(amount_in <= amount_in_max, ERR_INSUFFICIENT_INPUT_AMOUNT);
        // swap
        let coins_in = coin::withdraw<X>(account, amount_in);
        let coins_out;
        let coins_mid;
        coins_mid = swap_coins_for_coins<X, Y>(coins_in);
        coins_out = swap_coins_for_coins<Y, Z>(coins_mid);
        AnimeSwapPoolV1Library::register_coin<Z>(account);
        coin::deposit<Z>(signer::address_of(account), coins_out);
    }

    /// 3 pairs swap X->Y->Z->W
    public entry fun swap_coins_for_exact_coins_3_pair_entry<X, Y, Z, W>(
        account: &signer,
        amount_out: u64,
        amount_in_max: u64,
    ) acquires LiquidityPool, AdminData, Events {
        let amount_in = get_amounts_in_3_pair<X, Y, Z, W>(amount_out);
        assert!(amount_in <= amount_in_max, ERR_INSUFFICIENT_INPUT_AMOUNT);
        // swap
        let coins_in = coin::withdraw<X>(account, amount_in);
        let coins_out;
        let coins_mid;
        let coins_mid_2;
        coins_mid = swap_coins_for_coins<X, Y>(coins_in);
        coins_mid_2 = swap_coins_for_coins<Y, Z>(coins_mid);
        coins_out = swap_coins_for_coins<Z, W>(coins_mid_2);
        AnimeSwapPoolV1Library::register_coin<W>(account);
        coin::deposit<W>(signer::address_of(account), coins_out);
    }

    /**
     *  Setting config functions
     */

    public entry fun set_dao_fee_to(
        account: &signer,
        dao_fee_to: address
    ) acquires AdminData {
        let admin_data = borrow_global_mut<AdminData>(RESOURCE_ACCOUNT_ADDRESS);
        assert!(signer::address_of(account) == admin_data.admin_address, ERR_FORBIDDEN);
        admin_data.dao_fee_to = dao_fee_to;
    }

    public entry fun set_admin_address(
        account: &signer,
        admin_address: address
    ) acquires AdminData {
        let admin_data = borrow_global_mut<AdminData>(RESOURCE_ACCOUNT_ADDRESS);
        assert!(signer::address_of(account) == admin_data.admin_address, ERR_FORBIDDEN);
        admin_data.admin_address = admin_address;
    }

    public entry fun set_dao_fee(
        account: &signer,
        dao_fee: u8
    ) acquires AdminData {
        let admin_data = borrow_global_mut<AdminData>(RESOURCE_ACCOUNT_ADDRESS);
        assert!(signer::address_of(account) == admin_data.admin_address, ERR_FORBIDDEN);
        if (dao_fee == 0) {
            admin_data.dao_fee_on = false;
        } else {
            admin_data.dao_fee_on = true;
            admin_data.dao_fee = dao_fee;
        };
    }

    public entry fun set_swap_fee(
        account: &signer,
        swap_fee: u64
    ) acquires AdminData {
        let admin_data = borrow_global_mut<AdminData>(RESOURCE_ACCOUNT_ADDRESS);
        assert!(signer::address_of(account) == admin_data.admin_address, ERR_FORBIDDEN);
        assert!(swap_fee <= 1000, ERR_FORBIDDEN);
        admin_data.swap_fee = swap_fee;
    }

    public entry fun withdraw_dao_fee<X, Y>(
        account: &signer
    ) acquires AdminData {
        if (!AnimeSwapPoolV1Library::compare<X, Y>()) {
            withdraw_dao_fee<Y, X>(account);
            return
        };
        let admin_data = borrow_global<AdminData>(RESOURCE_ACCOUNT_ADDRESS);
        let acc_addr = signer::address_of(account);
        assert!(acc_addr == admin_data.dao_fee_to, ERR_FORBIDDEN);
        if (!coin::is_account_registered<LPCoin<X, Y>>(acc_addr)) {
            coin::register<LPCoin<X, Y>>(account);
        };
        let amount = coin::balance<LPCoin<X, Y>>(RESOURCE_ACCOUNT_ADDRESS) - MINIMUM_LIQUIDITY;
        coin::transfer<LPCoin<X, Y>>(&get_resource_account_signer(), acc_addr, amount);
    }

    /// pause swap, only remove lp is allowed
    /// EMERGENCY ONLY
    public entry fun pause(
        account: &signer
    ) acquires AdminData {
        assert_not_paused();
        let admin_data = borrow_global_mut<AdminData>(RESOURCE_ACCOUNT_ADDRESS);
        assert!(signer::address_of(account) == admin_data.admin_address, ERR_FORBIDDEN);
        admin_data.is_pause = true;
    }

    /// unpause swap
    /// EMERGENCY ONLY
    public entry fun unpause(
        account: &signer
    ) acquires AdminData {
        assert_paused();
        let admin_data = borrow_global_mut<AdminData>(RESOURCE_ACCOUNT_ADDRESS);
        assert!(signer::address_of(account) == admin_data.admin_address, ERR_FORBIDDEN);
        admin_data.is_pause = false;
    }

    /**
     *  Router functions, can be called by other contracts
     */

    /// Create pair, and register events
    /// require X < Y
    public fun create_pair<X, Y>() acquires AdminData, PairInfo {
        assert!(AnimeSwapPoolV1Library::compare<X, Y>(), ERR_PAIR_ORDER_ERROR);
        assert!(!exists<LiquidityPool<X, Y>>(RESOURCE_ACCOUNT_ADDRESS), ERR_PAIR_ALREADY_EXIST);
        assert_not_paused();
        let resource_account_signer = get_resource_account_signer();
        // create lp coin
        let (lp_b, lp_f, lp_m) = coin::initialize<LPCoin<X, Y>>(&resource_account_signer, utf8(b"AnimeSwapLPCoin"), utf8(b"ANILPCoin"), 8, true);
        // register coin
        AnimeSwapPoolV1Library::register_coin<LPCoin<X, Y>>(&resource_account_signer);
        // register LiquidityPool
        move_to(&resource_account_signer, LiquidityPool<X, Y>{
            coin_x_reserve: coin::zero<X>(),
            coin_y_reserve: coin::zero<Y>(),
            last_block_timestamp: 0,
            last_price_x_cumulative: 0,
            last_price_y_cumulative: 0,
            k_last: 0,
            lp_mint_cap: lp_m,
            lp_freeze_cap: lp_f,
            lp_burn_cap: lp_b,
            locked: false,
        });
        // add pair_info
        let pair_meta = get_pair_meta<X, Y>();
        let pair_info = borrow_global_mut<PairInfo>(RESOURCE_ACCOUNT_ADDRESS);
        vector::push_back<PairMeta>(&mut pair_info.pair_list, copy pair_meta);

        // init events
        let events = Events<X, Y> {
            pair_created_event: account::new_event_handle<PairCreatedEvent<X, Y>>(&resource_account_signer),
            mint_event: account::new_event_handle<MintEvent<X, Y>>(&resource_account_signer),
            burn_event: account::new_event_handle<BurnEvent<X, Y>>(&resource_account_signer),
            swap_event: account::new_event_handle<SwapEvent<X, Y>>(&resource_account_signer),
            sync_event: account::new_event_handle<SyncEvent<X, Y>>(&resource_account_signer),
            flash_swap_event: account::new_event_handle<FlashSwapEvent<X, Y>>(&resource_account_signer),
        };
        event::emit_event(&mut events.pair_created_event, PairCreatedEvent {
            meta: pair_meta,
        });
        move_to(&resource_account_signer, events);
    }

    /// Add liquidity
    /// require X < Y
    public fun add_liquidity<X, Y>(
        account: &signer,
        amount_x_desired: u64,
        amount_y_desired: u64,
        amount_x_min: u64,
        amount_y_min: u64,
    ) acquires LiquidityPool, AdminData, Events {
        // check lp exist
        assert!(AnimeSwapPoolV1Library::compare<X, Y>(), ERR_PAIR_ORDER_ERROR);
        assert!(exists<LiquidityPool<X, Y>>(RESOURCE_ACCOUNT_ADDRESS), ERR_PAIR_NOT_EXIST);
        let (amount_x, amount_y) = calc_optimal_coin_values<X, Y>(amount_x_desired, amount_y_desired, amount_x_min, amount_y_min);
        let coin_x = coin::withdraw<X>(account, amount_x);
        let coin_y = coin::withdraw<Y>(account, amount_y);
        let lp_coins = mint<X, Y>(coin_x, coin_y);

        let acc_addr = signer::address_of(account);
        if (!coin::is_account_registered<LPCoin<X, Y>>(acc_addr)) {
            coin::register<LPCoin<X, Y>>(account);
        };
        coin::deposit(acc_addr, lp_coins);
    }

    /// Remove liquidity
    /// require X < Y
    public fun remove_liquidity<X, Y>(
        coins: Coin<LPCoin<X, Y>>,
        amount_x_min: u64,
        amount_y_min: u64,
    ): (Coin<X>, Coin<Y>) acquires LiquidityPool, AdminData, Events {
        assert!(AnimeSwapPoolV1Library::compare<X, Y>(), ERR_PAIR_ORDER_ERROR);
        let (x_out, y_out) = burn<X, Y>(coins);
        assert!(coin::value(&x_out) >= amount_x_min, ERR_INSUFFICIENT_X_AMOUNT);
        assert!(coin::value(&y_out) >= amount_y_min, ERR_INSUFFICIENT_Y_AMOUNT);
        (x_out, y_out)
    }

    /// Swap X to Y
    /// swap from X to Y
    public fun swap_coins_for_coins<X, Y>(
        coins_in: Coin<X>,
    ): Coin<Y> acquires LiquidityPool, AdminData, Events {
        let amount_in = coin::value(&coins_in);
        let swap_fee = borrow_global<AdminData>(RESOURCE_ACCOUNT_ADDRESS).swap_fee;
        let (reserve_in, reserve_out) = get_reserves_size<X, Y>();
        let amount_out = AnimeSwapPoolV1Library::get_amount_out(amount_in, reserve_in, reserve_out, swap_fee);
        let (zero, coins_out);
        if (AnimeSwapPoolV1Library::compare<X, Y>()) {
            (zero, coins_out) = swap<X, Y>(coins_in, 0, coin::zero(), amount_out);
        } else {
            (coins_out, zero) = swap<Y, X>(coin::zero(), amount_out, coins_in, 0);
        };
        coin::destroy_zero<X>(zero);
        coins_out
    }

    /**
     *  Low level functions, can be called by other contracts
     */

    /// Mint new LPCoin
    public fun mint<X, Y>(
        coin_x: Coin<X>,
        coin_y: Coin<Y>
    ): Coin<LPCoin<X, Y>> acquires LiquidityPool, AdminData, Events {
        assert!(AnimeSwapPoolV1Library::compare<X, Y>(), ERR_PAIR_ORDER_ERROR);
        assert!(exists<LiquidityPool<X, Y>>(RESOURCE_ACCOUNT_ADDRESS), ERR_PAIR_NOT_EXIST);
        assert_not_paused();
        assert_lp_unlocked<X, Y>();

        let amount_x = coin::value(&coin_x);
        let amount_y = coin::value(&coin_y);
        // get reserve
        let lp = borrow_global_mut<LiquidityPool<X, Y>>(RESOURCE_ACCOUNT_ADDRESS);
        let (reserve_x, reserve_y) = (coin::value(&lp.coin_x_reserve), coin::value(&lp.coin_y_reserve));
        let admin_data = borrow_global<AdminData>(RESOURCE_ACCOUNT_ADDRESS);
        // feeOn
        let fee_on = mint_fee_interval<X, Y>(lp, admin_data);
        coin::merge(&mut lp.coin_x_reserve, coin_x);
        coin::merge(&mut lp.coin_y_reserve, coin_y);
        let (balance_x, balance_y) = (coin::value(&lp.coin_x_reserve), coin::value(&lp.coin_y_reserve));

        let total_supply = AnimeSwapPoolV1Library::get_lpcoin_total_supply<LPCoin<X, Y>>();
        let liquidity;
        if (total_supply == 0) {
            liquidity = AnimeSwapPoolV1Library::sqrt(amount_x, amount_y) - MINIMUM_LIQUIDITY;
            mint_coin<X, Y>(&get_resource_account_signer(), MINIMUM_LIQUIDITY, &lp.lp_mint_cap);
        } else {
            // normal tx should never overflow
            let amount_1 = ((amount_x as u128) * total_supply / (reserve_x as u128) as u64);
            let amount_2 = ((amount_y as u128) * total_supply / (reserve_y as u128) as u64);
            liquidity = AnimeSwapPoolV1Library::min(amount_1, amount_2);
        };
        assert!(liquidity > 0, ERR_INSUFFICIENT_LIQUIDITY_MINT);
        let coins = coin::mint<LPCoin<X, Y>>(liquidity, &lp.lp_mint_cap);
        // update interval
        update_internal(lp, balance_x, balance_y, reserve_x, reserve_y);
        // feeOn
        if (fee_on) lp.k_last = (balance_x as u128) * (balance_y as u128);
        // event
        let events = borrow_global_mut<Events<X, Y>>(RESOURCE_ACCOUNT_ADDRESS);
        event::emit_event(&mut events.mint_event, MintEvent {
            amount_x,
            amount_y,
            liquidity,
        });
        coins
    }

    /// Burn LPCoin and get back coins
    public fun burn<X, Y>(
        liquidity: Coin<LPCoin<X, Y>>
    ): (Coin<X>, Coin<Y>) acquires LiquidityPool, AdminData, Events {
        assert!(AnimeSwapPoolV1Library::compare<X, Y>(), ERR_PAIR_ORDER_ERROR);
        assert_lp_unlocked<X, Y>();
        let liquidity_amount = coin::value(&liquidity);
        // get lp
        let lp = borrow_global_mut<LiquidityPool<X, Y>>(RESOURCE_ACCOUNT_ADDRESS);
        let (reserve_x, reserve_y) = (coin::value(&lp.coin_x_reserve), coin::value(&lp.coin_y_reserve));
        let admin_data = borrow_global<AdminData>(RESOURCE_ACCOUNT_ADDRESS);
        // feeOn
        let fee_on = mint_fee_interval<X, Y>(lp, admin_data);

        let total_supply = AnimeSwapPoolV1Library::get_lpcoin_total_supply<LPCoin<X, Y>>();
        let amount_x = ((liquidity_amount as u128) * (reserve_x as u128) / total_supply as u64);
        let amount_y = ((liquidity_amount as u128) * (reserve_y as u128) / total_supply as u64);
        let x_coin_to_return = coin::extract(&mut lp.coin_x_reserve, amount_x);
        let y_coin_to_return = coin::extract(&mut lp.coin_y_reserve, amount_y);
        assert!(amount_x > 0 && amount_y > 0, ERR_INSUFFICIENT_LIQUIDITY_BURN);
        let (balance_x, balance_y) = (coin::value(&lp.coin_x_reserve), coin::value(&lp.coin_y_reserve));
        coin::burn<LPCoin<X, Y>>(liquidity, &lp.lp_burn_cap);

        // update interval
        update_internal(lp, balance_x, balance_y, reserve_x, reserve_y);
        // feeOn
        if (fee_on) lp.k_last = (balance_x as u128) * (balance_y as u128);
        // event
        let events = borrow_global_mut<Events<X, Y>>(RESOURCE_ACCOUNT_ADDRESS);
        event::emit_event(&mut events.burn_event, BurnEvent {
            amount_x,
            amount_y,
            liquidity: liquidity_amount,
        });
        (x_coin_to_return, y_coin_to_return)
    }

    /// Swap coins
    public fun swap<X, Y>(
        coins_x_in: Coin<X>,
        amount_x_out: u64,
        coins_y_in: Coin<Y>,
        amount_y_out: u64,
    ): (Coin<X>, Coin<Y>) acquires LiquidityPool, AdminData, Events {
        assert_not_paused();
        assert_lp_unlocked<X, Y>();
        let amount_x_in = coin::value(&coins_x_in);
        let amount_y_in = coin::value(&coins_y_in);
        assert!(amount_x_in > 0 || amount_y_in > 0, ERR_INSUFFICIENT_INPUT_AMOUNT);
        assert!(amount_x_out > 0 || amount_y_out > 0, ERR_INSUFFICIENT_OUTPUT_AMOUNT);
        let lp = borrow_global_mut<LiquidityPool<X, Y>>(RESOURCE_ACCOUNT_ADDRESS);
        let (reserve_x, reserve_y) = (coin::value(&lp.coin_x_reserve), coin::value(&lp.coin_y_reserve));
        coin::merge(&mut lp.coin_x_reserve, coins_x_in);
        coin::merge(&mut lp.coin_y_reserve, coins_y_in);
        let coins_x_out = coin::extract(&mut lp.coin_x_reserve, amount_x_out);
        let coins_y_out = coin::extract(&mut lp.coin_y_reserve, amount_y_out);
        let (balance_x, balance_y) = (coin::value(&lp.coin_x_reserve), coin::value(&lp.coin_y_reserve));
        assert_k_increase(balance_x, balance_y, amount_x_in, amount_y_in, reserve_x, reserve_y);
        // update internal
        update_internal(lp, balance_x, balance_y, reserve_x, reserve_y);
        // event
        let events = borrow_global_mut<Events<X, Y>>(RESOURCE_ACCOUNT_ADDRESS);
        event::emit_event(&mut events.swap_event, SwapEvent {
            amount_x_in,
            amount_y_in,
            amount_x_out,
            amount_y_out,
        });
        (coins_x_out, coins_y_out)
    }

    /**
     *  Misc public functions for other contract
     */

    /// price oracle for other contract
    public fun get_last_price_cumulative<X, Y>(): (u128, u128, u64) acquires LiquidityPool {
        if (AnimeSwapPoolV1Library::compare<X, Y>()) {
            let lp = borrow_global<LiquidityPool<X, Y>>(RESOURCE_ACCOUNT_ADDRESS);
            (lp.last_price_x_cumulative, lp.last_price_y_cumulative, lp.last_block_timestamp)
        } else {
            let lp = borrow_global<LiquidityPool<Y, X>>(RESOURCE_ACCOUNT_ADDRESS);
            (lp.last_price_y_cumulative, lp.last_price_x_cumulative, lp.last_block_timestamp)
        }
    }

    public fun check_pair_exist<X, Y>(): bool {
        if (AnimeSwapPoolV1Library::compare<X, Y>()) {
            exists<LiquidityPool<X, Y>>(RESOURCE_ACCOUNT_ADDRESS)
        } else {
            exists<LiquidityPool<Y, X>>(RESOURCE_ACCOUNT_ADDRESS)
        }
    }

    public fun get_admin_data(): (u64, u8, bool, bool) acquires AdminData {
        let admin_data = borrow_global<AdminData>(RESOURCE_ACCOUNT_ADDRESS);
        (admin_data.swap_fee, admin_data.dao_fee, admin_data.dao_fee_on, admin_data.is_pause)
    }

    public fun get_pair_list(): vector<PairMeta> acquires PairInfo {
        let pair_info = borrow_global<PairInfo>(RESOURCE_ACCOUNT_ADDRESS);
        pair_info.pair_list
    }

    /**
     *  Flash swap functions, be called by other contracts
     */

    /// Get flash swap coins. User can loan any coins, and repay in the same tx.
    /// In most cases, user may loan one coin, and repay the same or the other coin.
    /// require X < Y.
    /// * `loan_coin_x` - expected amount of X coins to loan.
    /// * `loan_coin_y` - expected amount of Y coins to loan.
    /// Returns both loaned X and Y coins: `(Coin<X>, Coin<Y>, Flashloan<X, Y)`.
    public fun flash_swap<X, Y>(
        loan_coin_x: u64,
        loan_coin_y: u64
    ): (Coin<X>, Coin<Y>, FlashSwap<X, Y>) acquires LiquidityPool, AdminData {
        // assert check
        assert!(AnimeSwapPoolV1Library::compare<X, Y>(), ERR_PAIR_ORDER_ERROR);
        assert!(loan_coin_x > 0 || loan_coin_y > 0, ERR_LOAN_ERROR);
        assert_not_paused();
        assert_lp_unlocked<X, Y>();

        let lp = borrow_global_mut<LiquidityPool<X, Y>>(RESOURCE_ACCOUNT_ADDRESS);
        assert!(coin::value(&lp.coin_x_reserve) >= loan_coin_x && coin::value(&lp.coin_y_reserve) >= loan_coin_y, ERR_INSUFFICIENT_AMOUNT);
        lp.locked = true;

        let loaned_coin_x = coin::extract(&mut lp.coin_x_reserve, loan_coin_x);
        let loaned_coin_y = coin::extract(&mut lp.coin_y_reserve, loan_coin_y);

        // Return loaned amount.
        (loaned_coin_x, loaned_coin_y, FlashSwap<X, Y> {loan_coin_x, loan_coin_y})
    }

    /// Repay flash swap coins.
    /// User should repay amount, following the conditions:
    /// `new_pool_1_value * new_pool_2_value >= old_pool_1_value * old_pool_2_value`
    /// where `new_pool_x_value` is the `old_pool_x_value - amount_out + amount_in * (1 - swapFee)`,
    /// and `pool_x_value` is the reserve amount for a given CoinType.
    /// * `x_in` - X coins to pay.
    /// * `y_in` - Y coins to pay.
    /// * `flash_swap` - flash_swap return.
    public fun pay_flash_swap<X, Y>(
        x_in: Coin<X>,
        y_in: Coin<Y>,
        flash_swap: FlashSwap<X, Y>
    ) acquires LiquidityPool, AdminData, Events {
        // assert check
        assert!(AnimeSwapPoolV1Library::compare<X, Y>(), ERR_PAIR_ORDER_ERROR);
        assert!(exists<LiquidityPool<X, Y>>(RESOURCE_ACCOUNT_ADDRESS), ERR_PAIR_NOT_EXIST);
        assert_not_paused();

        let FlashSwap { loan_coin_x, loan_coin_y } = flash_swap;
        let amount_x_in = coin::value(&x_in);
        let amount_y_in = coin::value(&y_in);

        assert!(amount_x_in > 0 || amount_y_in > 0, ERR_LOAN_ERROR);

        let lp = borrow_global_mut<LiquidityPool<X, Y>>(RESOURCE_ACCOUNT_ADDRESS);
        let reserve_x = coin::value(&lp.coin_x_reserve);
        let reserve_y = coin::value(&lp.coin_y_reserve);

        // reserve size before loan out
        reserve_x = reserve_x + loan_coin_x;
        reserve_y = reserve_y + loan_coin_y;

        coin::merge(&mut lp.coin_x_reserve, x_in);
        coin::merge(&mut lp.coin_y_reserve, y_in);

        let balance_x = coin::value(&lp.coin_x_reserve);
        let balance_y = coin::value(&lp.coin_y_reserve);
        assert_k_increase(balance_x, balance_y, amount_x_in, amount_y_in, reserve_x, reserve_y);
        // update internal
        update_internal(lp, balance_x, balance_y, reserve_x, reserve_y);

        lp.locked = false;
        // event
        let events = borrow_global_mut<Events<X, Y>>(RESOURCE_ACCOUNT_ADDRESS);
        event::emit_event(&mut events.flash_swap_event, FlashSwapEvent {
            loan_coin_x,
            loan_coin_y,
            repay_coin_x: amount_x_in,
            repay_coin_y: amount_y_in,
        });
    }

    #[test_only]
    use aptos_framework::genesis;
    #[test_only]
    use aptos_framework::account::create_account_for_test;
    #[test_only]
    const TEST_ERROR:u64 = 10000;
    #[test_only]
    const ADD_LIQUIDITY_ERROR:u64 = 10003;
    #[test_only]
    const CONTRACTOR_BALANCE_ERROR:u64 = 10004;
    #[test_only]
    const USER_LP_BALANCE_ERROR:u64 = 10005;
    #[test_only]
    const INIT_FAUCET_COIN:u64 = 1000000000;

    #[test_only]
    struct Aptos {}
    #[test_only]
    struct AptosB {}
    #[test_only]
    struct BTC {}
    #[test_only]
    struct USDT {}
    #[test_only]
    struct Caps<phantom X> has key {
        mint: MintCapability<X>,
        freeze: FreezeCapability<X>,
        burn: BurnCapability<X>,
    }

    #[test_only]
    fun init_module_test(resource_account: &signer) acquires AdminData, PairInfo {
        move_to(resource_account, AdminData {
            signer_cap: account::create_test_signer_cap(signer::address_of(resource_account)),
            dao_fee_to: DEPLOYER_ADDRESS,
            admin_address: DEPLOYER_ADDRESS,
            dao_fee: 5,         // 1/6 to dao fee
            swap_fee: 30,       // 0.3%
            dao_fee_on: false,  // default false
            is_pause: false,    // default false
        });
        move_to(resource_account, PairInfo{
            pair_list: vector::empty(),
        });
        // create default 3 pairs
        create_pair<BTC, USDT>();
        create_pair<std::aptos_coin::AptosCoin, BTC>();
        create_pair<std::aptos_coin::AptosCoin, USDT>();
    }

    #[test_only]
    fun test_init(creator: &signer, resource_account: &signer, someone_else: &signer) acquires AdminData, PairInfo {
        genesis::setup();
        create_account_for_test(signer::address_of(creator));
        create_account_for_test(signer::address_of(resource_account));
        create_account_for_test(signer::address_of(someone_else));
        init_module_test(resource_account);

        // init timestamp
        timestamp::update_global_time_for_test(100);

        {
            // init self-defined BTC
            let (apt_b, apt_f, apt_m) = coin::initialize<BTC>(creator, utf8(b"Bitcoin"), utf8(b"BTC"), 6, true);
            coin::register<BTC>(resource_account);
            coin::register<BTC>(someone_else);
            let coins = coin::mint<BTC>(INIT_FAUCET_COIN, &apt_m);
            coin::deposit(signer::address_of(someone_else), coins);
            move_to(resource_account, Caps<BTC> { mint: apt_m, freeze: apt_f, burn: apt_b });
        };

        {
            // init self-defined USDT
            let (apt_b, apt_f, apt_m) = coin::initialize<USDT>(creator, utf8(b"Tether"), utf8(b"USDT"), 6, true);
            coin::register<USDT>(resource_account);
            coin::register<USDT>(someone_else);
            let coins = coin::mint<USDT>(INIT_FAUCET_COIN, &apt_m);
            coin::deposit(signer::address_of(someone_else), coins);
            move_to(resource_account, Caps<USDT> { mint: apt_m, freeze: apt_f, burn: apt_b });
        };

        {
            // init self-defined Aptos
            let (apt_b, apt_f, apt_m) = coin::initialize<Aptos>(creator, utf8(b"Aptos"), utf8(b"APT"), 6, true);
            coin::register<Aptos>(resource_account);
            coin::register<Aptos>(someone_else);
            let coins = coin::mint<Aptos>(INIT_FAUCET_COIN, &apt_m);
            coin::deposit(signer::address_of(someone_else), coins);
            move_to(resource_account, Caps<Aptos> { mint: apt_m, freeze: apt_f, burn: apt_b });
        };

        {
            // init self-defined AptosB
            let (apt_b, apt_f, apt_m) = coin::initialize<AptosB>(creator, utf8(b"AptosB"), utf8(b"APTB"), 6, true);
            coin::register<AptosB>(resource_account);
            coin::register<AptosB>(someone_else);
            let coins = coin::mint<AptosB>(INIT_FAUCET_COIN, &apt_m);
            coin::deposit(signer::address_of(someone_else), coins);
            move_to(resource_account, Caps<AptosB> { mint: apt_m, freeze: apt_f, burn: apt_b });
        };

        create_pair<BTC, Aptos>();
        create_pair<USDT, Aptos>();
        create_pair<Aptos, AptosB>();
    }

    #[test_only]
    fun test_init_another_one(resource_account: &signer, another_one: &signer) acquires Caps {
        create_account_for_test(signer::address_of(another_one));
        {
            coin::register<BTC>(another_one);
            let caps = borrow_global<Caps<BTC>>(signer::address_of(resource_account));
            let coins = coin::mint<BTC>(INIT_FAUCET_COIN, &caps.mint);
            coin::deposit(signer::address_of(another_one), coins);
        };
        {
            coin::register<USDT>(another_one);
            let caps = borrow_global<Caps<USDT>>(signer::address_of(resource_account));
            let coins = coin::mint<USDT>(INIT_FAUCET_COIN, &caps.mint);
            coin::deposit(signer::address_of(another_one), coins);
        };
    }

    #[test(creator = @SwapDeployer, resource_account = @ResourceAccountDeployer, someone_else = @0x11)]
    public entry fun test_add_remove_liquidity_basic_1_1(creator: &signer, resource_account: &signer, someone_else: &signer) 
            acquires LiquidityPool, AdminData, PairInfo, Events {
        // init
        test_init(creator, resource_account, someone_else);

        // should takes 10000/10000 coin and gives 9000 LPCoin (AnimeSwapPoolV1Library::sqrt(10000*10000)-1000)
        add_liquidity_entry<BTC, USDT>(someone_else, 10000, 10000, 1, 1);
        {
            let lp = borrow_global<LiquidityPool<BTC, USDT>>(RESOURCE_ACCOUNT_ADDRESS);
            assert!(coin::value(&lp.coin_x_reserve) == 10000, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp.coin_y_reserve) == 10000, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::balance<LPCoin<BTC, USDT>>(signer::address_of(someone_else)) == 9000, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<BTC>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 10000, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<USDT>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 10000, USER_LP_BALANCE_ERROR);
        };

        // should takes 100/100 coin and gives 100 LPCoin
        add_liquidity_entry<BTC, USDT>(someone_else, 1000, 100, 1, 1);
        {
            let lp = borrow_global<LiquidityPool<BTC, USDT>>(RESOURCE_ACCOUNT_ADDRESS);
            assert!(coin::value(&lp.coin_x_reserve) == 10100, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp.coin_y_reserve) == 10100, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::balance<LPCoin<BTC, USDT>>(signer::address_of(someone_else)) == 9100, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<BTC>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 10100, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<USDT>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 10100, USER_LP_BALANCE_ERROR);
        };

        // should takes 9000 LPCoin and gives 9000/9000 coin
        remove_liquidity_entry<BTC, USDT>(someone_else, 9000, 9000, 9000);
        {
            let lp = borrow_global<LiquidityPool<BTC, USDT>>(RESOURCE_ACCOUNT_ADDRESS);
            assert!(coin::value(&lp.coin_x_reserve) == 1100, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp.coin_y_reserve) == 1100, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::balance<LPCoin<BTC, USDT>>(signer::address_of(someone_else)) == 100, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<BTC>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 1100, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<USDT>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 1100, USER_LP_BALANCE_ERROR);
        };
    }

    #[test(creator = @SwapDeployer, resource_account = @ResourceAccountDeployer, someone_else = @0x11)]
    public entry fun test_add_remove_liquidity_basic_1_100(creator: &signer, resource_account: &signer, someone_else: &signer) 
            acquires LiquidityPool, AdminData, PairInfo, Events {
        // init
        test_init(creator, resource_account, someone_else);

        // should takes 1000/100000 coin and gives 9000 LPCoin (AnimeSwapPoolV1Library::sqrt(1000*100000)-1000)
        add_liquidity_entry<BTC, USDT>(someone_else, 1000, 100000, 1, 1);
        {
            let lp = borrow_global<LiquidityPool<BTC, USDT>>(RESOURCE_ACCOUNT_ADDRESS);
            assert!(coin::value(&lp.coin_x_reserve) == 1000, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp.coin_y_reserve) == 100000, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::balance<LPCoin<BTC, USDT>>(signer::address_of(someone_else)) == 9000, USER_LP_BALANCE_ERROR);
        };

        // should takes 10/1000 coin and gives 100 LPCoin
        add_liquidity_entry<BTC, USDT>(someone_else, 1000, 1000, 1, 1);
        {
            let lp = borrow_global<LiquidityPool<BTC, USDT>>(RESOURCE_ACCOUNT_ADDRESS);
            assert!(coin::value(&lp.coin_x_reserve) == 1010, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp.coin_y_reserve) == 101000, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::balance<LPCoin<BTC, USDT>>(signer::address_of(someone_else)) == 9100, USER_LP_BALANCE_ERROR);
        };

        // should takes 9000 LPCoin and gives 900/90000 coin
        remove_liquidity_entry<BTC, USDT>(someone_else, 9000, 900, 90000);
        {
            let lp = borrow_global<LiquidityPool<BTC, USDT>>(RESOURCE_ACCOUNT_ADDRESS);
            assert!(coin::value(&lp.coin_x_reserve) == 110, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp.coin_y_reserve) == 11000, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::balance<LPCoin<BTC, USDT>>(signer::address_of(someone_else)) == 100, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<BTC>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 110, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<USDT>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 11000, USER_LP_BALANCE_ERROR);
        };
    }

    #[test(creator = @SwapDeployer, resource_account = @ResourceAccountDeployer, someone_else = @0x11)]
    public entry fun test_add_remove_liquidity_basic_1_2(creator: &signer, resource_account: &signer, someone_else: &signer) 
            acquires LiquidityPool, AdminData, PairInfo, Events {
        // init
        test_init(creator, resource_account, someone_else);

        // should takes 1000/2000 coin and gives 414 LPCoin (AnimeSwapPoolV1Library::sqrt(1000*2000)-1000)
        add_liquidity_entry<BTC, USDT>(someone_else, 1000, 2000, 1, 1);
        {
            let lp = borrow_global<LiquidityPool<BTC, USDT>>(RESOURCE_ACCOUNT_ADDRESS);
            assert!(coin::value(&lp.coin_x_reserve) == 1000, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp.coin_y_reserve) == 2000, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::balance<LPCoin<BTC, USDT>>(signer::address_of(someone_else)) == 414, USER_LP_BALANCE_ERROR);
        };

        // should takes 1000/2000 coin and gives 1414 LPCoin
        add_liquidity_entry<BTC, USDT>(someone_else, 2000, 2000, 1, 1);
        {
            let lp = borrow_global<LiquidityPool<BTC, USDT>>(RESOURCE_ACCOUNT_ADDRESS);
            assert!(coin::value(&lp.coin_x_reserve) == 2000, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp.coin_y_reserve) == 4000, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::balance<LPCoin<BTC, USDT>>(signer::address_of(someone_else)) == 1828, USER_LP_BALANCE_ERROR);
        };

        // should takes 1828 LPCoin and gives 1828/2828*2000=1292|1828/2828*4000=2585 coin
        remove_liquidity_entry<BTC, USDT>(someone_else, 1828, 1, 1);
        {
            let lp = borrow_global<LiquidityPool<BTC, USDT>>(RESOURCE_ACCOUNT_ADDRESS);
            assert!(coin::value(&lp.coin_x_reserve) == 2000-1292, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp.coin_y_reserve) == 4000-2585, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::balance<LPCoin<BTC, USDT>>(signer::address_of(someone_else)) == 0, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<BTC>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - (2000-1292), USER_LP_BALANCE_ERROR);
            assert!(coin::balance<USDT>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - (4000-2585), USER_LP_BALANCE_ERROR);
        };
    }

    #[test(creator = @SwapDeployer, resource_account = @ResourceAccountDeployer, someone_else = @0x11)]
    public entry fun test_swap_basic_1(creator: &signer, resource_account: &signer, someone_else: &signer) 
            acquires LiquidityPool, AdminData, PairInfo, Events {
        // init
        test_init(creator, resource_account, someone_else);

        // should takes 10000/10000 coin and gives 9000 LPCoin (AnimeSwapPoolV1Library::sqrt(10000*10000)-1000)
        add_liquidity_entry<BTC, USDT>(someone_else, 10000, 10000, 1, 1);
        {
            let lp = borrow_global<LiquidityPool<BTC, USDT>>(RESOURCE_ACCOUNT_ADDRESS);
            assert!(coin::value(&lp.coin_x_reserve) == 10000, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp.coin_y_reserve) == 10000, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::balance<LPCoin<BTC, USDT>>(signer::address_of(someone_else)) == 9000, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<BTC>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 10000, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<USDT>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 10000, USER_LP_BALANCE_ERROR);
        };

        swap_exact_coins_for_coins_entry<BTC, USDT>(someone_else, 1000, 1);
        {
            let lp = borrow_global<LiquidityPool<BTC, USDT>>(RESOURCE_ACCOUNT_ADDRESS);
            assert!(coin::value(&lp.coin_x_reserve) == 11000, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp.coin_y_reserve) == 9094, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::balance<LPCoin<BTC, USDT>>(signer::address_of(someone_else)) == 9000, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<BTC>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 10000 - 1000, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<USDT>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 10000 + 906, USER_LP_BALANCE_ERROR);
        };

        swap_exact_coins_for_coins_entry<USDT, BTC>(someone_else, 1000, 1);
        {
            let lp = borrow_global<LiquidityPool<BTC, USDT>>(RESOURCE_ACCOUNT_ADDRESS);
            assert!(coin::value(&lp.coin_x_reserve) == 9914, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp.coin_y_reserve) == 10094, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::balance<LPCoin<BTC, USDT>>(signer::address_of(someone_else)) == 9000, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<BTC>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 10000 - 1000 + 1086, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<USDT>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 10000 + 906 - 1000, USER_LP_BALANCE_ERROR);
        };

        // should takes 1000 LPCoin and gives 1000/10000*9914=991|1000/10000*10094=1009 coin
        remove_liquidity_entry<BTC, USDT>(someone_else, 1000, 1, 1);
        {
            let lp = borrow_global<LiquidityPool<BTC, USDT>>(RESOURCE_ACCOUNT_ADDRESS);
            assert!(coin::value(&lp.coin_x_reserve) == 9914 - 991, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp.coin_y_reserve) == 10094 - 1009, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::balance<LPCoin<BTC, USDT>>(signer::address_of(someone_else)) == 8000, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<BTC>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 10000 - 1000 + 1086 + 991, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<USDT>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 10000 + 906 - 1000 + 1009, USER_LP_BALANCE_ERROR);
        };
    }

    #[test(creator = @SwapDeployer, resource_account = @ResourceAccountDeployer, someone_else = @0x11)]
    public entry fun test_swap_basic_2(creator: &signer, resource_account: &signer, someone_else: &signer)
            acquires LiquidityPool, AdminData, PairInfo, Events {
        // init
        test_init(creator, resource_account, someone_else);

        // should takes 10000/10000 coin and gives 9000 LPCoin (AnimeSwapPoolV1Library::sqrt(10000*10000)-1000)
        add_liquidity_entry<BTC, USDT>(someone_else, 10000, 10000, 1, 1);
        {
            let lp = borrow_global<LiquidityPool<BTC, USDT>>(RESOURCE_ACCOUNT_ADDRESS);
            assert!(coin::value(&lp.coin_x_reserve) == 10000, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp.coin_y_reserve) == 10000, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::balance<LPCoin<BTC, USDT>>(signer::address_of(someone_else)) == 9000, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<BTC>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 10000, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<USDT>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 10000, USER_LP_BALANCE_ERROR);
        };

        swap_coins_for_exact_coins_entry<BTC, USDT>(someone_else, 1000, 100000);
        {
            let lp = borrow_global<LiquidityPool<BTC, USDT>>(RESOURCE_ACCOUNT_ADDRESS);
            assert!(coin::value(&lp.coin_x_reserve) == 11115, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp.coin_y_reserve) == 9000, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::balance<LPCoin<BTC, USDT>>(signer::address_of(someone_else)) == 9000, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<BTC>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 10000 - 1115, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<USDT>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 10000 + 1000, USER_LP_BALANCE_ERROR);
        };

        swap_coins_for_exact_coins_entry<USDT, BTC>(someone_else, 1000, 100000);
        {
            let lp = borrow_global<LiquidityPool<BTC, USDT>>(RESOURCE_ACCOUNT_ADDRESS);
            assert!(coin::value(&lp.coin_x_reserve) == 10115, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp.coin_y_reserve) == 9893, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::balance<LPCoin<BTC, USDT>>(signer::address_of(someone_else)) == 9000, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<BTC>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 10000 - 1115 + 1000, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<USDT>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 10000 + 1000 - 893, USER_LP_BALANCE_ERROR);
        };
    }

    #[test(creator = @SwapDeployer, resource_account = @ResourceAccountDeployer, someone_else = @0x11)]
    public entry fun test_swap_multiple_pair_1_1(creator: &signer, resource_account: &signer, someone_else: &signer)
            acquires LiquidityPool, AdminData, PairInfo, Events {
        // init
        test_init(creator, resource_account, someone_else);

        // should takes 10000/10000 coin and gives 9000 LPCoin (AnimeSwapPoolV1Library::sqrt(10000*10000)-1000)
        add_liquidity_entry<BTC, USDT>(someone_else, 10000000, 10000000, 1, 1);
        add_liquidity_entry<USDT, Aptos>(someone_else, 10000000, 10000000, 1, 1);

        swap_exact_coins_for_coins_2_pair_entry<BTC, USDT, Aptos>(someone_else, 10000, 1);
        {
            let lp = borrow_global<LiquidityPool<BTC, USDT>>(RESOURCE_ACCOUNT_ADDRESS);
            let lp_2 = borrow_global<LiquidityPool<USDT, Aptos>>(RESOURCE_ACCOUNT_ADDRESS);
            assert!(coin::value(&lp.coin_x_reserve) == 10010000, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp.coin_y_reserve) == 9990040, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp_2.coin_x_reserve) == 10009960, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp_2.coin_y_reserve) == 9990080, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::balance<BTC>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 10000000 - 10000, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<USDT>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 20000000, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<Aptos>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 10000000 + 9920, USER_LP_BALANCE_ERROR);
        };
    }

    #[test(creator = @SwapDeployer, resource_account = @ResourceAccountDeployer, someone_else = @0x11)]
    public entry fun test_swap_multiple_pair_1_2(creator: &signer, resource_account: &signer, someone_else: &signer)
            acquires LiquidityPool, AdminData, PairInfo, Events {
        // init
        test_init(creator, resource_account, someone_else);

        // should takes 10000/10000 coin and gives 9000 LPCoin (AnimeSwapPoolV1Library::sqrt(10000*10000)-1000)
        add_liquidity_entry<BTC, USDT>(someone_else, 10000000, 10000000, 1, 1);
        add_liquidity_entry<USDT, Aptos>(someone_else, 10000000, 10000000, 1, 1);
        add_liquidity_entry<Aptos, AptosB>(someone_else, 10000000, 10000000, 1, 1);

        swap_exact_coins_for_coins_3_pair_entry<BTC, USDT, Aptos, AptosB>(someone_else, 10000, 1);
        {
            let lp = borrow_global<LiquidityPool<BTC, USDT>>(RESOURCE_ACCOUNT_ADDRESS);
            let lp_2 = borrow_global<LiquidityPool<USDT, Aptos>>(RESOURCE_ACCOUNT_ADDRESS);
            let lp_3 = borrow_global<LiquidityPool<Aptos, AptosB>>(RESOURCE_ACCOUNT_ADDRESS);
            assert!(coin::value(&lp.coin_x_reserve) == 10010000, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp.coin_y_reserve) == 9990040, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp_2.coin_x_reserve) == 10009960, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp_2.coin_y_reserve) == 9990080, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp_3.coin_x_reserve) == 10009920, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp_3.coin_y_reserve) == 9990120, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::balance<BTC>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 10000000 - 10000, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<USDT>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 20000000, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<Aptos>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 20000000, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<AptosB>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 10000000 + 9880, USER_LP_BALANCE_ERROR);
        };
    }

    #[test(creator = @SwapDeployer, resource_account = @ResourceAccountDeployer, someone_else = @0x11)]
    public entry fun test_swap_multiple_pair_2_1(creator: &signer, resource_account: &signer, someone_else: &signer)
            acquires LiquidityPool, AdminData, PairInfo, Events {
        // init
        test_init(creator, resource_account, someone_else);
        // std::aptos_coin::mint(signer::address_of(someone_else), 10000000);

        // should takes 10000/10000 coin and gives 9000 LPCoin (AnimeSwapPoolV1Library::sqrt(10000*10000)-1000)
        add_liquidity_entry<BTC, USDT>(someone_else, 10000000, 10000000, 1, 1);
        add_liquidity_entry<USDT, Aptos>(someone_else, 10000000, 10000000, 1, 1);

        swap_coins_for_exact_coins_2_pair_entry<BTC, USDT, Aptos>(someone_else, 10000, 1000000);
        {
            let lp = borrow_global<LiquidityPool<BTC, USDT>>(RESOURCE_ACCOUNT_ADDRESS);
            let lp_2 = borrow_global<LiquidityPool<USDT, Aptos>>(RESOURCE_ACCOUNT_ADDRESS);
            assert!(coin::value(&lp.coin_x_reserve) == 10010082, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp.coin_y_reserve) == 9989959, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp_2.coin_x_reserve) == 10010041, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp_2.coin_y_reserve) == 9990000, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::balance<BTC>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 10000000 - 10082, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<USDT>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 20000000, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<Aptos>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 10000000 + 10000, USER_LP_BALANCE_ERROR);
        }
    }

    #[test(creator = @SwapDeployer, resource_account = @ResourceAccountDeployer, someone_else = @0x11)]
    public entry fun test_swap_multiple_pair_2_1_1(creator: &signer, resource_account: &signer, someone_else: &signer)
            acquires LiquidityPool, AdminData, PairInfo, Events {
        // init
        test_init(creator, resource_account, someone_else);

        add_liquidity_entry<BTC, USDT>(someone_else, 100000000, 1000000, 1, 1);
        add_liquidity_entry<USDT, Aptos>(someone_else, 1000000, 1000000, 1, 1);

        swap_coins_for_exact_coins_2_pair_entry<BTC, USDT, Aptos>(someone_else, 10000, 100000000000000);
        {
            let lp = borrow_global<LiquidityPool<BTC, USDT>>(RESOURCE_ACCOUNT_ADDRESS);
            let lp_2 = borrow_global<LiquidityPool<USDT, Aptos>>(RESOURCE_ACCOUNT_ADDRESS);
            assert!(coin::value(&lp.coin_x_reserve) == 101026651, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp.coin_y_reserve) == 989868, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp_2.coin_x_reserve) == 1010132, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp_2.coin_y_reserve) == 990000, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::balance<BTC>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 100000000 - 1026651, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<USDT>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 2000000, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<Aptos>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 1000000 + 10000, USER_LP_BALANCE_ERROR);
        }
    }

    #[test(creator = @SwapDeployer, resource_account = @ResourceAccountDeployer, someone_else = @0x11)]
    public entry fun test_swap_multiple_pair_2_2(creator: &signer, resource_account: &signer, someone_else: &signer)
            acquires LiquidityPool, AdminData, PairInfo, Events {
        // init
        test_init(creator, resource_account, someone_else);

        // should takes 10000/10000 coin and gives 9000 LPCoin (AnimeSwapPoolV1Library::sqrt(10000*10000)-1000)
        add_liquidity_entry<BTC, USDT>(someone_else, 10000000, 10000000, 1, 1);
        add_liquidity_entry<USDT, Aptos>(someone_else, 10000000, 10000000, 1, 1);
        add_liquidity_entry<Aptos, AptosB>(someone_else, 10000000, 10000000, 1, 1);

        swap_coins_for_exact_coins_3_pair_entry<BTC, USDT, Aptos, AptosB>(someone_else, 10000, 1000000);
        {
            let lp = borrow_global<LiquidityPool<BTC, USDT>>(RESOURCE_ACCOUNT_ADDRESS);
            let lp_2 = borrow_global<LiquidityPool<USDT, Aptos>>(RESOURCE_ACCOUNT_ADDRESS);
            let lp_3 = borrow_global<LiquidityPool<Aptos, AptosB>>(RESOURCE_ACCOUNT_ADDRESS);
            assert!(coin::value(&lp.coin_x_reserve) == 10010123, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp.coin_y_reserve) == 9989918, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp_2.coin_x_reserve) == 10010082, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp_2.coin_y_reserve) == 9989959, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp_3.coin_x_reserve) == 10010041, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp_3.coin_y_reserve) == 9990000, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::balance<BTC>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 10000000 - 10123, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<USDT>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 20000000, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<Aptos>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 20000000, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<AptosB>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 10000000 + 10000, USER_LP_BALANCE_ERROR);
        };
    }

    #[test(creator = @SwapDeployer, resource_account = @ResourceAccountDeployer, someone_else = @0x11)]
    public entry fun test_swap_multiple_pair_2_2_1(creator: &signer, resource_account: &signer, someone_else: &signer)
            acquires LiquidityPool, AdminData, PairInfo, Events {
        // init
        test_init(creator, resource_account, someone_else);

        add_liquidity_entry<BTC, USDT>(someone_else, 10000000, 1000000, 1, 1);
        add_liquidity_entry<USDT, Aptos>(someone_else, 1000000, 100000, 1, 1);
        add_liquidity_entry<Aptos, AptosB>(someone_else, 100000, 100000, 1, 1);

        swap_coins_for_exact_coins_3_pair_entry<BTC, USDT, Aptos, AptosB>(someone_else, 1000, 100000000000000);
        {
            let lp = borrow_global<LiquidityPool<BTC, USDT>>(RESOURCE_ACCOUNT_ADDRESS);
            let lp_2 = borrow_global<LiquidityPool<USDT, Aptos>>(RESOURCE_ACCOUNT_ADDRESS);
            let lp_3 = borrow_global<LiquidityPool<Aptos, AptosB>>(RESOURCE_ACCOUNT_ADDRESS);
            assert!(coin::value(&lp.coin_x_reserve) == 10104130, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp.coin_y_reserve) == 989725, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp_2.coin_x_reserve) == 1010275, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp_2.coin_y_reserve) == 98986, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp_3.coin_x_reserve) == 101014, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp_3.coin_y_reserve) == 99000, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::balance<BTC>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 10000000 - 104130, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<USDT>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 2000000, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<Aptos>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 200000, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<AptosB>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 100000 + 1000, USER_LP_BALANCE_ERROR);
        };
    }

    // test remove more than expected
    #[test(creator = @SwapDeployer, resource_account = @ResourceAccountDeployer, someone_else = @0x11)]
    #[expected_failure(abort_code = 65542, location = coin)]
    public entry fun test_add_remove_liquidity_error_1(creator: &signer, resource_account: &signer, someone_else: &signer) 
            acquires LiquidityPool, AdminData, PairInfo, Events {
        // init
        test_init(creator, resource_account, someone_else);

        // should takes 10000/10000 coin and gives 9000 LPCoin (AnimeSwapPoolV1Library::sqrt(10000*10000)-1000)
        add_liquidity_entry<BTC, USDT>(someone_else, 10000, 10000, 1, 1);

        // only have 9000 LP, should fail
        remove_liquidity_entry<BTC, USDT>(someone_else, 9200, 9200, 9200);
    }

    // test remove more than expected
    #[test(creator = @SwapDeployer, resource_account = @ResourceAccountDeployer, someone_else = @0x11)]
    #[expected_failure(abort_code = ERR_INSUFFICIENT_Y_AMOUNT)]
    public entry fun test_add_remove_liquidity_error_2(creator: &signer, resource_account: &signer, someone_else: &signer) 
            acquires LiquidityPool, AdminData, PairInfo, Events {
        // init
        test_init(creator, resource_account, someone_else);

        // should takes 10000/10000 coin and gives 9000 LPCoin (AnimeSwapPoolV1Library::sqrt(10000*10000)-1000)
        add_liquidity_entry<BTC, USDT>(someone_else, 10000, 10000, 1, 1);

        // should takes 9000 LPCoin and gives 900/90000 coin, but expect more
        remove_liquidity_entry<BTC, USDT>(someone_else, 9000, 1000, 100000);
    }

    #[test(creator = @SwapDeployer, resource_account = @ResourceAccountDeployer, someone_else = @0x11)]
    public entry fun test_add_multiple_liquidity(creator: &signer, resource_account: &signer, someone_else: &signer)
            acquires LiquidityPool, AdminData, PairInfo, Events {
        // init
        test_init(creator, resource_account, someone_else);

        // add 3 LPs
        add_liquidity_entry<USDT, Aptos>(someone_else, 1000000, 10000, 1, 1);
        add_liquidity_entry<BTC, Aptos>(someone_else, 10000, 10000, 1, 1);
        add_liquidity_entry<BTC, USDT>(someone_else, 10000, 1000000, 1, 1);

        {
            let lp = borrow_global<LiquidityPool<USDT, Aptos>>(RESOURCE_ACCOUNT_ADDRESS);
            let lp_2 = borrow_global<LiquidityPool<BTC, Aptos>>(RESOURCE_ACCOUNT_ADDRESS);
            let lp_3 = borrow_global<LiquidityPool<BTC, USDT>>(RESOURCE_ACCOUNT_ADDRESS);
            assert!(coin::value(&lp.coin_x_reserve) == 1000000, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp.coin_y_reserve) == 10000, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp_2.coin_x_reserve) == 10000, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp_2.coin_y_reserve) == 10000, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp_3.coin_x_reserve) == 10000, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp_3.coin_y_reserve) == 1000000, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::balance<LPCoin<USDT, Aptos>>(signer::address_of(someone_else)) == 99000, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<LPCoin<BTC, Aptos>>(signer::address_of(someone_else)) == 9000, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<LPCoin<BTC, USDT>>(signer::address_of(someone_else)) == 99000, USER_LP_BALANCE_ERROR);
        };

        let lp_1 = borrow_global<LiquidityPool<USDT, Aptos>>(RESOURCE_ACCOUNT_ADDRESS);
        assert!(coin::value(&lp_1.coin_x_reserve) == 1000000, CONTRACTOR_BALANCE_ERROR);
        assert!(coin::value(&lp_1.coin_y_reserve) == 10000, CONTRACTOR_BALANCE_ERROR);
        let lp_2 = borrow_global<LiquidityPool<BTC, Aptos>>(RESOURCE_ACCOUNT_ADDRESS);
        assert!(coin::value(&lp_2.coin_x_reserve) == 10000, CONTRACTOR_BALANCE_ERROR);
        assert!(coin::value(&lp_2.coin_y_reserve) == 10000, CONTRACTOR_BALANCE_ERROR);
        let lp_3 = borrow_global<LiquidityPool<BTC, USDT>>(RESOURCE_ACCOUNT_ADDRESS);
        assert!(coin::value(&lp_3.coin_x_reserve) == 10000, CONTRACTOR_BALANCE_ERROR);
        assert!(coin::value(&lp_3.coin_y_reserve) == 1000000, CONTRACTOR_BALANCE_ERROR);

        // add 3 LPs
        add_liquidity_entry<USDT, Aptos>(someone_else, 1000000, 10000, 1, 1);
        add_liquidity_entry<BTC, Aptos>(someone_else, 10000, 10000, 1, 1);
        add_liquidity_entry<BTC, USDT>(someone_else, 10000, 1000000, 1, 1);

        {
            let lp = borrow_global<LiquidityPool<USDT, Aptos>>(RESOURCE_ACCOUNT_ADDRESS);
            let lp_2 = borrow_global<LiquidityPool<BTC, Aptos>>(RESOURCE_ACCOUNT_ADDRESS);
            let lp_3 = borrow_global<LiquidityPool<BTC, USDT>>(RESOURCE_ACCOUNT_ADDRESS);
            assert!(coin::value(&lp.coin_x_reserve) == 2000000, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp.coin_y_reserve) == 20000, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp_2.coin_x_reserve) == 20000, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp_2.coin_y_reserve) == 20000, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp_3.coin_x_reserve) == 20000, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp_3.coin_y_reserve) == 2000000, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::balance<LPCoin<USDT, Aptos>>(signer::address_of(someone_else)) == 199000, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<LPCoin<BTC, Aptos>>(signer::address_of(someone_else)) == 19000, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<LPCoin<BTC, USDT>>(signer::address_of(someone_else)) == 199000, USER_LP_BALANCE_ERROR);
        };

        let lp_1 = borrow_global<LiquidityPool<USDT, Aptos>>(RESOURCE_ACCOUNT_ADDRESS);
        assert!(coin::value(&lp_1.coin_x_reserve) == 2000000, CONTRACTOR_BALANCE_ERROR);
        assert!(coin::value(&lp_1.coin_y_reserve) == 20000, CONTRACTOR_BALANCE_ERROR);
        let lp_2 = borrow_global<LiquidityPool<BTC, Aptos>>(RESOURCE_ACCOUNT_ADDRESS);
        assert!(coin::value(&lp_2.coin_x_reserve) == 20000, CONTRACTOR_BALANCE_ERROR);
        assert!(coin::value(&lp_2.coin_y_reserve) == 20000, CONTRACTOR_BALANCE_ERROR);
        let lp_3 = borrow_global<LiquidityPool<BTC, USDT>>(RESOURCE_ACCOUNT_ADDRESS);
        assert!(coin::value(&lp_3.coin_x_reserve) == 20000, CONTRACTOR_BALANCE_ERROR);
        assert!(coin::value(&lp_3.coin_y_reserve) == 2000000, CONTRACTOR_BALANCE_ERROR);
    }

    #[test(creator = @SwapDeployer, resource_account = @ResourceAccountDeployer, someone_else = @0x11, dao_fee_to = @0x99)]
    public entry fun test_dao_fee(creator: &signer, resource_account: &signer, someone_else: &signer, dao_fee_to: &signer)
            acquires LiquidityPool, AdminData, PairInfo, Events {
        // init
        test_init(creator, resource_account, someone_else);
        create_account_for_test(signer::address_of(dao_fee_to));
        set_dao_fee(creator, 1);
        set_dao_fee_to(creator, signer::address_of(dao_fee_to));

        // should takes 10000/10000 coin and gives 9000 LPCoin (AnimeSwapPoolV1Library::sqrt(10000*10000)-1000)
        add_liquidity_entry<BTC, USDT>(someone_else, 100000000, 100000000, 1, 1);
        {
            let lp = borrow_global<LiquidityPool<BTC, USDT>>(RESOURCE_ACCOUNT_ADDRESS);
            assert!(coin::value(&lp.coin_x_reserve) == 100000000, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp.coin_y_reserve) == 100000000, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::balance<LPCoin<BTC, USDT>>(signer::address_of(someone_else)) == 100000000 - 1000, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<BTC>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 100000000, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<USDT>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 100000000, USER_LP_BALANCE_ERROR);
        };

        swap_exact_coins_for_coins_entry<BTC, USDT>(someone_else, 10000000, 1);
        {
            let lp = borrow_global<LiquidityPool<BTC, USDT>>(RESOURCE_ACCOUNT_ADDRESS);
            assert!(coin::value(&lp.coin_x_reserve) == 110000000, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp.coin_y_reserve) == 90933892, CONTRACTOR_BALANCE_ERROR);   // 1e8-floor(1e8-1e8**2/(1e8+0.0997e8))
            assert!(coin::balance<LPCoin<BTC, USDT>>(signer::address_of(someone_else)) == 100000000 - 1000, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<BTC>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 100000000 - 10000000, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<USDT>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 100000000 + 9066108, USER_LP_BALANCE_ERROR);
        };

        // admin_data should have some dao LPCoins
        remove_liquidity_entry<BTC, USDT>(someone_else, 1000000, 1, 1);
        {
            let lp = borrow_global<LiquidityPool<BTC, USDT>>(RESOURCE_ACCOUNT_ADDRESS);
            assert!(coin::value(&lp.coin_x_reserve) == 108900076, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp.coin_y_reserve) == 90024616, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::balance<LPCoin<BTC, USDT>>(signer::address_of(someone_else)) == 98999000, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<BTC>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 108900076, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<USDT>(signer::address_of(someone_else)) == INIT_FAUCET_COIN - 90024616, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<LPCoin<BTC, USDT>>(RESOURCE_ACCOUNT_ADDRESS) == 1000 + 6819, USER_LP_BALANCE_ERROR);
        };

        // dao withdraw fee
        withdraw_dao_fee<BTC, USDT>(dao_fee_to);
        {
            assert!(coin::balance<LPCoin<BTC, USDT>>(RESOURCE_ACCOUNT_ADDRESS) == 1000, USER_LP_BALANCE_ERROR);
            assert!(coin::balance<LPCoin<BTC, USDT>>(signer::address_of(dao_fee_to)) == 6819, USER_LP_BALANCE_ERROR);
        };
    }

    // test resource account equal
    #[test(deployer = @SwapDeployer)]
    public entry fun test_resource_account(deployer: &signer) {
        genesis::setup();
        create_account_for_test(signer::address_of(deployer));
        let addr = account::create_resource_address(&signer::address_of(deployer), x"30");
        assert!(addr == @ResourceAccountDeployer, TEST_ERROR);
    }

    // borrow on boin and repay the other coin, greater than swap fee
    // borrow BTC and repay USDT
    #[test(creator = @SwapDeployer, resource_account = @ResourceAccountDeployer, someone_else = @0x11, another_one = @0x12)]
    public entry fun test_flash_swap_a(creator: &signer, resource_account: &signer, someone_else: &signer, another_one : &signer)
            acquires LiquidityPool, AdminData, PairInfo, Caps, Events {
        // init
        test_init(creator, resource_account, someone_else);
        test_init_another_one(resource_account, another_one);

        // if swap 1000 coin, should be 10000-1000/100000+11145 remain
        add_liquidity_entry<BTC, USDT>(someone_else, 10000, 100000, 1, 1);
        let amount_out = 1000;
        let amount_in = get_amounts_in_1_pair<USDT, BTC>(amount_out);
        assert!(amount_in == 11145, TEST_ERROR);
        let (coin_out_1, coin_out_2, flash_swap) = flash_swap<BTC, USDT>(amount_out, 0);
        coin::deposit<BTC>(signer::address_of(another_one), coin_out_1);
        coin::deposit<USDT>(signer::address_of(another_one), coin_out_2);
        let repay_coin = coin::withdraw<USDT>(another_one, amount_in);
        pay_flash_swap<BTC, USDT>(coin::zero<BTC>(), repay_coin, flash_swap);
        {
            let lp = borrow_global<LiquidityPool<BTC, USDT>>(RESOURCE_ACCOUNT_ADDRESS);
            assert!(coin::value(&lp.coin_x_reserve) == 10000 - 1000, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp.coin_y_reserve) == 100000 + 11145, CONTRACTOR_BALANCE_ERROR);
        };
    }

    // borrow on boin and repay the other coin, greater than swap fee
    // borrow USDT and repay BTC
    #[test(creator = @SwapDeployer, resource_account = @ResourceAccountDeployer, someone_else = @0x11, another_one = @0x12)]
    public entry fun test_flash_swap_b(creator: &signer, resource_account: &signer, someone_else: &signer, another_one : &signer)
            acquires LiquidityPool, AdminData, PairInfo, Caps, Events {
        // init
        test_init(creator, resource_account, someone_else);
        test_init_another_one(resource_account, another_one);

        // if swap 1000 coin, should be 10000+102/100000-1000 remain
        add_liquidity_entry<BTC, USDT>(someone_else, 10000, 100000, 1, 1);
        let amount_out = 1000;
        let amount_in = get_amounts_in_1_pair<BTC, USDT>(amount_out);
        assert!(amount_in == 102, TEST_ERROR);
        let (coin_out_1, coin_out_2, flash_swap) = flash_swap<BTC, USDT>(0, amount_out);
        coin::deposit<BTC>(signer::address_of(another_one), coin_out_1);
        coin::deposit<USDT>(signer::address_of(another_one), coin_out_2);
        let repay_coin = coin::withdraw<BTC>(another_one, amount_in);
        pay_flash_swap<BTC, USDT>(repay_coin, coin::zero<USDT>(), flash_swap);
        {
            let lp = borrow_global<LiquidityPool<BTC, USDT>>(RESOURCE_ACCOUNT_ADDRESS);
            assert!(coin::value(&lp.coin_x_reserve) == 10000 + 102, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp.coin_y_reserve) == 100000 - 1000, CONTRACTOR_BALANCE_ERROR);
        };
    }

    // ERR_K_ERROR, not enough coin repay
    // borrow on boin and repay the other coin but less equal than swap fee
    #[test(creator = @SwapDeployer, resource_account = @ResourceAccountDeployer, someone_else = @0x11, another_one = @0x12)]
    #[expected_failure(abort_code = ERR_K_ERROR)]
    public entry fun test_flash_swap_error(creator: &signer, resource_account: &signer, someone_else: &signer, another_one : &signer)
            acquires LiquidityPool, AdminData, PairInfo, Caps, Events {
        // init
        test_init(creator, resource_account, someone_else);
        test_init_another_one(resource_account, another_one);

        // if swap 1000 coin, should be 9000/11115 remain
        add_liquidity_entry<BTC, USDT>(someone_else, 10000, 10000, 1, 1);
        let (coin_out_1, coin_out_2, flash_swap) = flash_swap<BTC, USDT>(1000, 0);
        coin::deposit<BTC>(signer::address_of(another_one), coin_out_1);
        coin::deposit<USDT>(signer::address_of(another_one), coin_out_2);
        let repay_coin = coin::withdraw<USDT>(another_one, 1114);
        pay_flash_swap<BTC, USDT>(coin::zero<BTC>(), repay_coin, flash_swap);
    }

    // borrow both boins and repay greater than swap fee
    #[test(creator = @SwapDeployer, resource_account = @ResourceAccountDeployer, someone_else = @0x11, another_one = @0x12)]
    public entry fun test_flash_swap_2(creator: &signer, resource_account: &signer, someone_else: &signer, another_one : &signer)
            acquires LiquidityPool, AdminData, PairInfo, Caps, Events {
        // init
        test_init(creator, resource_account, someone_else);
        test_init_another_one(resource_account, another_one);

        add_liquidity_entry<BTC, USDT>(someone_else, 10000, 10000, 1, 1);
        let (coin_out_1, coin_out_2, flash_swap) = flash_swap<BTC, USDT>(1000, 1000);
        coin::deposit<BTC>(signer::address_of(another_one), coin_out_1);
        coin::deposit<USDT>(signer::address_of(another_one), coin_out_2);
        let repay_coin_x = coin::withdraw<BTC>(another_one, 1004);
        let repay_coin_y = coin::withdraw<USDT>(another_one, 1003);
        pay_flash_swap<BTC, USDT>(repay_coin_x, repay_coin_y, flash_swap);
        {
            let lp = borrow_global<LiquidityPool<BTC, USDT>>(RESOURCE_ACCOUNT_ADDRESS);
            assert!(coin::value(&lp.coin_x_reserve) == 10004, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp.coin_y_reserve) == 10003, CONTRACTOR_BALANCE_ERROR);
        };
    }

    // ERR_K_ERROR, not enough coin repay
    // borrow both boins and repay less equal than swap fee
    #[test(creator = @SwapDeployer, resource_account = @ResourceAccountDeployer, someone_else = @0x11, another_one = @0x12)]
    #[expected_failure(abort_code = ERR_K_ERROR)]
    public entry fun test_flash_swap_error_2(creator: &signer, resource_account: &signer, someone_else: &signer, another_one : &signer)
            acquires LiquidityPool, AdminData, PairInfo, Caps, Events {
        // init
        test_init(creator, resource_account, someone_else);
        test_init_another_one(resource_account, another_one);

        add_liquidity_entry<BTC, USDT>(someone_else, 10000, 10000, 1, 1);
        let (coin_out_1, coin_out_2, flash_swap) = flash_swap<BTC, USDT>(1000, 1000);
        coin::deposit<BTC>(signer::address_of(another_one), coin_out_1);
        coin::deposit<USDT>(signer::address_of(another_one), coin_out_2);
        let repay_coin_x = coin::withdraw<BTC>(another_one, 1003);
        let repay_coin_y = coin::withdraw<USDT>(another_one, 1003);
        pay_flash_swap<BTC, USDT>(repay_coin_x, repay_coin_y, flash_swap);
    }

    // borrow one boin and repay the same coin, greater than swap fee
    #[test(creator = @SwapDeployer, resource_account = @ResourceAccountDeployer, someone_else = @0x11, another_one = @0x12)]
    public entry fun test_flash_swap_3(creator: &signer, resource_account: &signer, someone_else: &signer, another_one : &signer)
            acquires LiquidityPool, AdminData, PairInfo, Caps, Events {
        // init
        test_init(creator, resource_account, someone_else);
        test_init_another_one(resource_account, another_one);

        add_liquidity_entry<BTC, USDT>(someone_else, 10000, 10000, 1, 1);
        let (coin_out_1, coin_out_2, flash_swap) = flash_swap<BTC, USDT>(1000, 0);
        coin::deposit<BTC>(signer::address_of(another_one), coin_out_1);
        coin::deposit<USDT>(signer::address_of(another_one), coin_out_2);
        let repay_coin_x = coin::withdraw<BTC>(another_one, 1004);
        pay_flash_swap<BTC, USDT>(repay_coin_x, coin::zero<USDT>(), flash_swap);
        {
            let lp = borrow_global<LiquidityPool<BTC, USDT>>(RESOURCE_ACCOUNT_ADDRESS);
            assert!(coin::value(&lp.coin_x_reserve) == 10004, CONTRACTOR_BALANCE_ERROR);
            assert!(coin::value(&lp.coin_y_reserve) == 10000, CONTRACTOR_BALANCE_ERROR);
        };
    }

    // ERR_K_ERROR, not enough coin repay
    // borrow one boin and repay the same coin, but less equal than swap fee
    #[test(creator = @SwapDeployer, resource_account = @ResourceAccountDeployer, someone_else = @0x11, another_one = @0x12)]
    #[expected_failure(abort_code = ERR_K_ERROR)]
    public entry fun test_flash_swap_error_3(creator: &signer, resource_account: &signer, someone_else: &signer, another_one : &signer)
            acquires LiquidityPool, AdminData, PairInfo, Caps, Events {
        // init
        test_init(creator, resource_account, someone_else);
        test_init_another_one(resource_account, another_one);

        add_liquidity_entry<BTC, USDT>(someone_else, 10000, 10000, 1, 1);
        let (coin_out_1, coin_out_2, flash_swap) = flash_swap<BTC, USDT>(1000, 0);
        coin::deposit<BTC>(signer::address_of(another_one), coin_out_1);
        coin::deposit<USDT>(signer::address_of(another_one), coin_out_2);
        let repay_coin_x = coin::withdraw<BTC>(another_one, 1003);
        pay_flash_swap<BTC, USDT>(repay_coin_x, coin::zero<USDT>(), flash_swap);
    }

    #[test(creator = @SwapDeployer, resource_account = @ResourceAccountDeployer, someone_else = @0x11, another_one = @0x12)]
    public entry fun test_tmp(creator: &signer, resource_account: &signer, someone_else: &signer, another_one : &signer)
            acquires LiquidityPool, AdminData, PairInfo, Caps, Events {
        // init
        test_init(creator, resource_account, someone_else);
        test_init_another_one(resource_account, another_one);

        // if swap 1000 coin, should be 10000-1000/100000+11145 remain
        add_liquidity_entry<BTC, USDT>(someone_else, 10000, 100000, 1, 1);
        let amount_out = 1000;
        let amount_in = get_amounts_in_1_pair<USDT, BTC>(amount_out);
        assert!(amount_in == 11145, TEST_ERROR);
        let coins_in = coin::withdraw<USDT>(another_one, amount_in);
        let coins_out = swap_coins_for_coins<USDT, BTC>(coins_in);
        assert!(coin::value(&coins_out) >= amount_out, TEST_ERROR);
        coin::deposit<BTC>(signer::address_of(another_one), coins_out)
    }
}