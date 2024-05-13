module resource_account::liquidity_pair {
    use aptos_std::smart_table::{Self, SmartTable};
    use aptos_std::signer;
    use aptos_framework::coin;
    use aptos_framework::aptos_account;
    use aptos_framework::aptos_coin::{AptosCoin};
    use aptos_framework::object::{Object};
    use aptos_framework::event;
    use aptos_framework::fungible_asset::{Self, Metadata};
    use aptos_framework::primary_fungible_store;
    // FA-supported DEX
    use swap::router;
    use swap::liquidity_pool;
    use swap::coin_wrapper;
    // Friend.
    use resource_account::resource_signer_holder;
    use resource_account::bonding_curve_launchpad;
	friend bonding_curve_launchpad;

    const FA_DECIMALS: u8 = 8;
    const INITIAL_VIRTUAL_APT_LIQUIDITY: u128 = 50_000_000_000;
    const APT_LIQUIDITY_THRESHOLD: u128 = 600_000_000_000;

    const EFA_PRIMARY_STORE_DOES_NOT_EXIST: u64 = 12;
    const ELIQUIDITY_PAIR_EXISTS_ALREADY: u64 = 100;
    const ELIQUIDITY_PAIR_DOES_NOT_EXIST: u64 = 101;
    const ELIQUIDITY_PAIR_DISABLED: u64 = 102;
    const ELIQUIDITY_PAIR_SWAP_AMOUNTOUT_INSIGNIFICANT: u64 = 111;

    //---------------------------Events---------------------------
    #[event]
    struct LiquidityPairCreated has store, drop {
        fa_obj: Object<Metadata>,
        initial_fa_reserves: u128,
        initial_apt_reserves: u128,
        k: u256
    }
    #[event]
    struct LiquidityPairReservesUpdated has store, drop {
        old_fa_reserves: u128,
        old_apt_reserves: u128,
        new_fa_reserves: u128,
        new_apt_reserves: u128
    }
    #[event]
    struct LiquidityPairSwap has store, drop {
        is_fa_else_apt: bool,
        gained: u128,
        swapper_address: address
    }
    #[event]
    struct LiquidityPairGraduated has store, drop {
        fa_obj: Object<Metadata>,
        dex_address: address
    }

    //---------------------------Structs---------------------------
    struct LiquidityPairSmartTable has key {
        liquidity_pairs: SmartTable<Object<Metadata>, LiquidityPair>
    }
    struct LiquidityPair has store {
        is_enabled: bool,
        is_frozen: bool,
        fa_reserves: u128,
        apt_reserves: u128,
        k_constant: u256
    }

    //---------------------------Init---------------------------
    fun init_module(account: &signer) {
        let liquidity_pair_table: LiquidityPairSmartTable = LiquidityPairSmartTable {
            liquidity_pairs: smart_table::new()
        };

        move_to(account, liquidity_pair_table);
    }


    //---------------------------Liquidity Pair---------------------------
    public(friend) fun register_liquidity_pair(transfer_ref: &fungible_asset::TransferRef, account: &signer, fa_metadata: Object<Metadata>, apt_initialPurchaseAmountIn: u64, fa_initialLiquidity: u128) acquires LiquidityPairSmartTable {
        let liquidity_pair_smartTable = borrow_global_mut<LiquidityPairSmartTable>(@resource_account);
        assert!(!smart_table::contains(&mut liquidity_pair_smartTable.liquidity_pairs, fa_metadata), ELIQUIDITY_PAIR_EXISTS_ALREADY);
        //* Initial APT reserves are virtual liquidity, for less extreme initial swaps.
        let k_constant: u256 = (fa_initialLiquidity as u256) * (INITIAL_VIRTUAL_APT_LIQUIDITY as u256);
        let initial_liquidity_pair = LiquidityPair {
            is_enabled: true,
            is_frozen: true,
            fa_reserves: fa_initialLiquidity,
            apt_reserves: INITIAL_VIRTUAL_APT_LIQUIDITY,
            k_constant: k_constant
        };
        smart_table::add(
            &mut liquidity_pair_smartTable.liquidity_pairs,
            fa_metadata,
            initial_liquidity_pair
        );

        event::emit(LiquidityPairCreated {
            fa_obj: fa_metadata,
            initial_fa_reserves: fa_initialLiquidity,
            initial_apt_reserves: INITIAL_VIRTUAL_APT_LIQUIDITY,
            k: k_constant
        });

        if(apt_initialPurchaseAmountIn != 0) {
            internal_swap_apt_to_fa(transfer_ref, account, fa_metadata, apt_initialPurchaseAmountIn);
        }
    }

    public(friend) fun internal_swap_fa_to_apt(transfer_ref: &fungible_asset::TransferRef, account: &signer,  fa_metadata: Object<Metadata>,  amountIn: u64) acquires LiquidityPairSmartTable {
        let liquidity_pair_smartTable = borrow_global_mut<LiquidityPairSmartTable>(@resource_account);
        assert!(smart_table::contains(&mut liquidity_pair_smartTable.liquidity_pairs, fa_metadata), ELIQUIDITY_PAIR_DOES_NOT_EXIST);
        let liquidity_pair = smart_table::borrow_mut(&mut liquidity_pair_smartTable.liquidity_pairs, fa_metadata);
        assert!(liquidity_pair.is_enabled, ELIQUIDITY_PAIR_DISABLED);

        let swapper_address = signer::address_of(account);
        let (fa_given, apt_gained, fa_updated_reserves, apt_updated_reserves) = get_amount_out(
            liquidity_pair.fa_reserves,
            liquidity_pair.apt_reserves,
            true,
            amountIn
        );
        let does_primary_store_exist_for_swapper = primary_fungible_store::primary_store_exists(swapper_address, fa_metadata);
        assert!(does_primary_store_exist_for_swapper, EFA_PRIMARY_STORE_DOES_NOT_EXIST);
        let account_address = signer::address_of(account);
        primary_fungible_store::transfer_with_ref(transfer_ref, swapper_address, @resource_account, fa_given);
        aptos_account::transfer(&resource_signer_holder::get_signer(), account_address, apt_gained);

        let old_fa_reserves = liquidity_pair.fa_reserves;
        let old_apt_reserves = liquidity_pair.apt_reserves;
        liquidity_pair.fa_reserves = fa_updated_reserves;
        liquidity_pair.apt_reserves = apt_updated_reserves;

        event::emit(LiquidityPairReservesUpdated {
            old_fa_reserves: old_fa_reserves,
            old_apt_reserves: old_apt_reserves,
            new_fa_reserves: fa_updated_reserves,
            new_apt_reserves: apt_updated_reserves
        });
        event::emit(LiquidityPairSwap {
            is_fa_else_apt: false,
            gained: (apt_gained as u128),
            swapper_address: swapper_address
        });
    }

    public(friend) fun internal_swap_apt_to_fa(transfer_ref: &fungible_asset::TransferRef, account: &signer,  fa_metadata: Object<Metadata>, amountIn: u64) acquires LiquidityPairSmartTable {
        let liquidity_pair_smartTable = borrow_global_mut<LiquidityPairSmartTable>(@resource_account);
        assert!(smart_table::contains(&mut liquidity_pair_smartTable.liquidity_pairs, fa_metadata), ELIQUIDITY_PAIR_DOES_NOT_EXIST);
        let liquidity_pair = smart_table::borrow_mut(&mut liquidity_pair_smartTable.liquidity_pairs, fa_metadata);
        assert!(liquidity_pair.is_enabled, ELIQUIDITY_PAIR_DISABLED);

        let swapper_address = signer::address_of(account);
        let (fa_gained, apt_given, fa_updated_reserves, apt_updated_reserves) = get_amount_out(
            liquidity_pair.fa_reserves,
            liquidity_pair.apt_reserves,
            false,
            amountIn
        );
        let does_primary_store_exist_for_swapper = primary_fungible_store::primary_store_exists(swapper_address, fa_metadata);
        if(!does_primary_store_exist_for_swapper){
            primary_fungible_store::create_primary_store(swapper_address, fa_metadata);
        };
        aptos_account::transfer(account, @resource_account, apt_given);
        primary_fungible_store::transfer_with_ref(transfer_ref, @resource_account, swapper_address, fa_gained);
        // Disable transfers from users.
        let old_fa_reserves = liquidity_pair.fa_reserves;
        let old_apt_reserves = liquidity_pair.apt_reserves;
        liquidity_pair.fa_reserves = fa_updated_reserves;
        liquidity_pair.apt_reserves = apt_updated_reserves;

        event::emit(LiquidityPairReservesUpdated {
            old_fa_reserves: old_fa_reserves,
            old_apt_reserves: old_apt_reserves,
            new_fa_reserves: fa_updated_reserves,
            new_apt_reserves: apt_updated_reserves
        });
        event::emit(LiquidityPairSwap {
            is_fa_else_apt: true,
            gained: (fa_gained as u128),
            swapper_address: swapper_address
        });

        if(apt_updated_reserves > APT_LIQUIDITY_THRESHOLD && liquidity_pair.is_enabled){
            // Disable Bonding Curve Launchpad pair.
            liquidity_pair.is_enabled = false;
            liquidity_pair.is_frozen = false;
            // Offload onto third party, public DEX.
            router::create_pool_coin<AptosCoin>(fa_metadata, false);
            add_liquidity_coin_entry_transfer_ref<AptosCoin>(transfer_ref, &resource_signer_holder::get_signer(), fa_metadata, false, ((apt_updated_reserves - (apt_updated_reserves / 10)) as u64), ((fa_updated_reserves - (fa_updated_reserves / 10)) as u64), 0, 0);
            // Send liquidity tokens to dead address.
            let apt_coin_wrapped = coin_wrapper::get_wrapper<AptosCoin>();
            let liquidity_obj = liquidity_pool::liquidity_pool(apt_coin_wrapped, fa_metadata, false);
            liquidity_pool::transfer(&resource_signer_holder::get_signer(), liquidity_obj, @0xdead, primary_fungible_store::balance(@resource_account, liquidity_obj));

            event::emit(LiquidityPairGraduated {
                fa_obj: fa_metadata,
                dex_address: @swap
            });
        }
    }

    //---------------------------DEX-helpers---------------------------
    fun add_liquidity_coin_entry_transfer_ref<CoinType>(
        transfer_ref: &fungible_asset::TransferRef,
        lp: &signer,
        token_2: Object<Metadata>,
        is_stable: bool,
        amount_1_desired: u64,
        amount_2_desired: u64,
        amount_1_min: u64,
        amount_2_min: u64,
    ) {
        let token_1 = coin_wrapper::get_wrapper<CoinType>();
        let (optimal_amount_1, optimal_amount_2, _) = router::optimal_liquidity_amounts(
            token_1,
            token_2,
            is_stable,
            amount_1_desired,
            amount_2_desired,
            amount_1_min,
            amount_2_min,
        );
        let optimal_1 = coin::withdraw<CoinType>(lp, optimal_amount_1);
        let optimal_2 = primary_fungible_store::withdraw_with_ref(transfer_ref, signer::address_of(lp), optimal_amount_2);
        router::add_liquidity_coin<CoinType>(lp, optimal_1, optimal_2, is_stable);
    }

    //---------------------------Views---------------------------
    #[view]
    public fun get_amount_out(fa_reserves: u128, apt_reserves: u128, supplied_fa_else_apt: bool, amountIn: u64): (u64, u64, u128, u128) {
        if (supplied_fa_else_apt) {
            let top = (apt_reserves as u256) * (amountIn as u256);
            let bot = (fa_reserves as u256) + (amountIn as u256);
            let apt_gained: u64 = ((top/bot) as u64);
            assert!(apt_gained > 0, ELIQUIDITY_PAIR_SWAP_AMOUNTOUT_INSIGNIFICANT);
            return (amountIn, apt_gained, fa_reserves+(amountIn as u128), apt_reserves-(apt_gained as u128))
        }
        else {
            let top = (fa_reserves as u256) * (amountIn as u256);
            let bot = (apt_reserves as u256) + (amountIn as u256);
            let fa_gained: u64 = ((top/bot) as u64);
            assert!(fa_gained > 0, ELIQUIDITY_PAIR_SWAP_AMOUNTOUT_INSIGNIFICANT);
            return (fa_gained, amountIn, fa_reserves-(fa_gained as u128), apt_reserves+(amountIn as u128))
        }

    }
    #[view]
    public fun get_is_frozen_metadata(fa_metadata: Object<Metadata>):bool acquires LiquidityPairSmartTable {
        let liquidity_pair_smartTable = borrow_global<LiquidityPairSmartTable>(@resource_account);
        assert!(smart_table::contains(&liquidity_pair_smartTable.liquidity_pairs, fa_metadata), ELIQUIDITY_PAIR_DOES_NOT_EXIST);
        smart_table::borrow(&liquidity_pair_smartTable.liquidity_pairs, fa_metadata).is_frozen
    }


    //---------------------------Tests---------------------------
    #[test_only]
    public fun initialize_for_test(deployer: &signer){
        let liquidity_pair_smartTable: LiquidityPairSmartTable = LiquidityPairSmartTable {
            liquidity_pairs: smart_table::new()
        };
        move_to(deployer, liquidity_pair_smartTable);
    }

    //---------------------------View Tests---------------------------
    #[test(deployer = @resource_account)]
    #[expected_failure(abort_code = ELIQUIDITY_PAIR_SWAP_AMOUNTOUT_INSIGNIFICANT)]
    public fun test_insignificant_fa_swap(deployer: &signer) {
        initialize_for_test(deployer);
        get_amount_out(1_000_000_000, 1_000_000_000, true, 0);
    }
    #[test(deployer = @resource_account)]
    #[expected_failure(abort_code = ELIQUIDITY_PAIR_SWAP_AMOUNTOUT_INSIGNIFICANT)]
    public fun test_insignificant_apt_swap(deployer: &signer) {
        initialize_for_test(deployer);
        get_amount_out(1_000_000_000, 1_000_000_000, false, 0);
    }

}
