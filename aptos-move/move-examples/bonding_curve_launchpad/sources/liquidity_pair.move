module resource_account::liquidity_pair {
    use aptos_std::smart_table::{Self, SmartTable};
    use aptos_std::signer;
    use aptos_std::math128;
    use aptos_framework::coin;
    use aptos_framework::aptos_account;
    use aptos_framework::aptos_coin::{AptosCoin};
    use aptos_framework::object::{Object};
    use aptos_framework::event;
    use aptos_framework::fungible_asset::{Metadata, TransferRef};
    use aptos_framework::primary_fungible_store;
    use swap::router;
    use swap::liquidity_pool;
    use swap::coin_wrapper;
    use resource_account::resource_signer_holder;
    use resource_account::bonding_curve_launchpad;
    friend bonding_curve_launchpad;

    const FA_DECIMALS: u8 = 8;
    const INITIAL_VIRTUAL_APT_LIQUIDITY: u128 = 50_000_000_000;
    const APT_LIQUIDITY_THRESHOLD: u128 = 600_000_000_000;

    /// Swapper does not own the FA being swapped.
    const EFA_PRIMARY_STORE_DOES_NOT_EXIST: u64 = 12;
    /// Liquidity pair (APT/FA) being created already exists.
    const ELIQUIDITY_PAIR_EXISTS_ALREADY: u64 = 100;
    /// Liquidity pair does not exist.
    const ELIQUIDITY_PAIR_DOES_NOT_EXIST: u64 = 101;
    /// Liquidity pair has trading disabled. Graduation has already occurred.
    const ELIQUIDITY_PAIR_DISABLED: u64 = 102;
    /// Swap results in negligible amount out. Requires increasing amount in.
    const ELIQUIDITY_PAIR_SWAP_AMOUNTOUT_INSIGNIFICANT: u64 = 111;

    //---------------------------Events---------------------------
    #[event]
    struct LiquidityPairCreated has store, drop {
        fa_object_metadata: Object<Metadata>,
        initial_fa_reserves: u128,
        initial_apt_reserves: u128
    }

    #[event]
    struct LiquidityPairReservesUpdated has store, drop {
        former_fa_reserves: u128,
        former_apt_reserves: u128,
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
        fa_object_metadata: Object<Metadata>,
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
        apt_reserves: u128
    }

    //---------------------------Init---------------------------
    fun init_module(account: &signer) {
        move_to(account, LiquidityPairSmartTable { liquidity_pairs: smart_table::new() });
    }

    //---------------------------Liquidity Pair---------------------------
    /// Creates a unique liquidity pair between a given FA and APT.
    /// Only callable from `bonding_curve_launchpad`.
    public(friend) fun register_liquidity_pair(
        transfer_ref: &TransferRef,
        account: &signer,
        fa_object_metadata: Object<Metadata>,
        apt_amount_in: u64,
        fa_initial_liquidity: u128
    ) acquires LiquidityPairSmartTable {
        // Only allow for creation of new APT-FA pairs.
        let liquidity_pair_smart_table =
            borrow_global_mut<LiquidityPairSmartTable>(@resource_account);
        assert!(!smart_table::contains(&liquidity_pair_smart_table.liquidity_pairs,
                fa_object_metadata),
            ELIQUIDITY_PAIR_EXISTS_ALREADY);
        // Define and store the state of the liquidity pair as:
        // Reserves, global frozen status (`is_frozen`), and enabled trading (`is_enabled`).
        // Initial APT reserves are virtual liquidity, for less extreme initial swaps (avoiding early adopter's
        // advantage, for fairness). README covers this topic in more depth.
        smart_table::add(&mut liquidity_pair_smart_table.liquidity_pairs,
            fa_object_metadata,
            LiquidityPair {
                is_enabled: true,
                is_frozen: true,
                fa_reserves: fa_initial_liquidity,
                apt_reserves: INITIAL_VIRTUAL_APT_LIQUIDITY
            });

        event::emit(LiquidityPairCreated {
                fa_object_metadata,
                initial_fa_reserves: fa_initial_liquidity,
                initial_apt_reserves: INITIAL_VIRTUAL_APT_LIQUIDITY
            });

        // Optional initial swap given to the creator of the FA.
        if (apt_amount_in > 0) {
            internal_swap_apt_to_fa(transfer_ref, account, fa_object_metadata, apt_amount_in);
        }
    }

    /// Facilitate swapping between a given FA to APT.
    public(friend) fun internal_swap_fa_to_apt(
        transfer_ref: &TransferRef,
        account: &signer,
        fa_object_metadata: Object<Metadata>,
        amount_in: u64
    ) acquires LiquidityPairSmartTable {
        // Verify the liquidity pair exists and is enabled for trading.
        let liquidity_pair_smart_table =
            borrow_global_mut<LiquidityPairSmartTable>(@resource_account);
        assert!(smart_table::contains(&liquidity_pair_smart_table.liquidity_pairs,
                fa_object_metadata),
            ELIQUIDITY_PAIR_DOES_NOT_EXIST);
        let liquidity_pair = smart_table::borrow_mut(&mut liquidity_pair_smart_table.liquidity_pairs,
            fa_object_metadata);
        assert!(liquidity_pair.is_enabled, ELIQUIDITY_PAIR_DISABLED);
        // Determine the amount received of APT, when given swapper-supplied amount_in of FA.
        let swapper_address = signer::address_of(account);
        let (fa_given, apt_gained, fa_updated_reserves, apt_updated_reserves) = get_amount_out(
            liquidity_pair.fa_reserves, liquidity_pair.apt_reserves, true, amount_in);
        // Verify the swapper holds the FA.
        let does_primary_store_exist_for_swapper = primary_fungible_store::primary_store_exists(
            swapper_address, fa_object_metadata);
        assert!(does_primary_store_exist_for_swapper, EFA_PRIMARY_STORE_DOES_NOT_EXIST);
        // Perform the swap.
        // Swapper sends FA to the liquidity pair. The liquidity pair sends APT to the swapper, in return.
        let account_address = signer::address_of(account);
        primary_fungible_store::transfer_with_ref(transfer_ref, swapper_address,
            @resource_account, fa_given);
        aptos_account::transfer(&resource_signer_holder::get_signer(), account_address,
            apt_gained);
        // Record state changes to the liquidity pair's reserves.
        let former_fa_reserves = liquidity_pair.fa_reserves;
        let former_apt_reserves = liquidity_pair.apt_reserves;
        liquidity_pair.fa_reserves = fa_updated_reserves;
        liquidity_pair.apt_reserves = apt_updated_reserves;

        event::emit(LiquidityPairReservesUpdated {
                former_fa_reserves,
                former_apt_reserves,
                new_fa_reserves: fa_updated_reserves,
                new_apt_reserves: apt_updated_reserves
            });
        event::emit(LiquidityPairSwap {
                is_fa_else_apt: false,
                gained: (apt_gained as u128),
                swapper_address
            });
    }

    /// Facilitate swapping between APT to a given FA.
    public(friend) fun internal_swap_apt_to_fa(
        transfer_ref: &TransferRef,
        account: &signer,
        fa_object_metadata: Object<Metadata>,
        amount_in: u64
    ) acquires LiquidityPairSmartTable {
        // Verify the liquidity pair exists and is enabled for trading.
        let liquidity_pair_smart_table =
            borrow_global_mut<LiquidityPairSmartTable>(@resource_account);
        assert!(smart_table::contains(&liquidity_pair_smart_table.liquidity_pairs,
                fa_object_metadata),
            ELIQUIDITY_PAIR_DOES_NOT_EXIST);
        let liquidity_pair = smart_table::borrow_mut(&mut liquidity_pair_smart_table.liquidity_pairs,
            fa_object_metadata);
        assert!(liquidity_pair.is_enabled, ELIQUIDITY_PAIR_DISABLED);
        // Determine the amount received of FA, when given swapper-supplied amount_in of APT.
        let swapper_address = signer::address_of(account);
        let (fa_gained, apt_given, fa_updated_reserves, apt_updated_reserves) = get_amount_out(
            liquidity_pair.fa_reserves, liquidity_pair.apt_reserves, false, amount_in);
        // Create the primary store for the swapper, if they don't already have one for the FA.
        let does_primary_store_exist_for_swapper = primary_fungible_store::primary_store_exists(
            swapper_address, fa_object_metadata);
        if (!does_primary_store_exist_for_swapper) {
            primary_fungible_store::create_primary_store(swapper_address, fa_object_metadata);
        };
        // Perform the swap.
        // Swapper sends APT to the liquidity pair. The liquidity pair sends FA to the swapper, in return.
        aptos_account::transfer(account, @resource_account, apt_given);
        primary_fungible_store::transfer_with_ref(transfer_ref, @resource_account,
            swapper_address, fa_gained);
        // Record state changes to the liquidity pair's reserves.
        let former_fa_reserves = liquidity_pair.fa_reserves;
        let former_apt_reserves = liquidity_pair.apt_reserves;
        liquidity_pair.fa_reserves = fa_updated_reserves;
        liquidity_pair.apt_reserves = apt_updated_reserves;

        event::emit(LiquidityPairReservesUpdated {
                former_fa_reserves,
                former_apt_reserves,
                new_fa_reserves: fa_updated_reserves,
                new_apt_reserves: apt_updated_reserves
            });
        event::emit(LiquidityPairSwap {
                is_fa_else_apt: true,
                gained: (fa_gained as u128),
                swapper_address
            });

        // Check for graduation requirements. The APT reserves must be above the pre-defined
        // threshold to allow for graduation.
        if (liquidity_pair.is_enabled && apt_updated_reserves > APT_LIQUIDITY_THRESHOLD) {
            graduate(liquidity_pair,
                fa_object_metadata,
                transfer_ref,
                apt_updated_reserves,
                fa_updated_reserves);
        }
    }

    /// Moves the reserves of a liquidity pair on the `liquidity_pair` module to a newly created liquidity pair
    /// on an external DEX (`swap`). The resulting liquidity provider tokens are thrown away.
    /// Both of the FA's original liquidity pair and frozen status are disabled.
    /// From here, participants are free to use their FA as they'd like, because the custom `withdraw` function is no
    /// longer frozen for `transfer`.
    fun graduate(
        liquidity_pair: &mut LiquidityPair,
        fa_object_metadata: Object<Metadata>,
        transfer_ref: &TransferRef,
        apt_updated_reserves: u128,
        fa_updated_reserves: u128
    ) {
        // Disable Bonding Curve Launchpad pair and remove global freeze on FA.
        liquidity_pair.is_enabled = false;
        liquidity_pair.is_frozen = false;
        // Offload onto third party, public DEX.
        router::create_pool_coin<AptosCoin>(fa_object_metadata, false);
        add_liquidity_coin_entry_transfer_ref<AptosCoin>(transfer_ref,
            &resource_signer_holder::get_signer(),
            fa_object_metadata,
            false,
            ((apt_updated_reserves - (apt_updated_reserves / 10)) as u64),
            ((fa_updated_reserves - (fa_updated_reserves / 10)) as u64),
            0,
            0);
        // Send liquidity tokens to dead address.
        let apt_coin_wrapped = coin_wrapper::get_wrapper<AptosCoin>();
        let liquidity_obj = liquidity_pool::liquidity_pool(apt_coin_wrapped,
            fa_object_metadata, false);
        liquidity_pool::transfer(&resource_signer_holder::get_signer(),
            liquidity_obj,
            @0xdead,
            primary_fungible_store::balance(@resource_account, liquidity_obj));

        event::emit(LiquidityPairGraduated { fa_object_metadata, dex_address: @swap });
    }

    //---------------------------DEX-helpers---------------------------
    /// Add liquidity alternative that relies on `transfer_ref`, rather than the traditional transfer
    /// found in the swap DEX.
    fun add_liquidity_coin_entry_transfer_ref<CoinType>(
        transfer_ref: &TransferRef,
        lp: &signer,
        token_2: Object<Metadata>,
        is_stable: bool,
        amount_1_desired: u64,
        amount_2_desired: u64,
        amount_1_min: u64,
        amount_2_min: u64,
    ) {
        // Wrap APT into a FA. Then, determine the optimal amounts for providing liquidity to the given FA - APT pair.
        let token_1 = coin_wrapper::get_wrapper<CoinType>();
        let (optimal_amount_1, optimal_amount_2, _) = router::optimal_liquidity_amounts(token_1,
            token_2,
            is_stable,
            amount_1_desired,
            amount_2_desired,
            amount_1_min,
            amount_2_min,);
        // Retrieve the APT and FA from the liquidity provider.
        // `transfer_ref` is used to avoid circular dependency during graduation. A normal transfer would require
        // visiting `bonding_curve_launchpad` to execute the custom withdraw logic. `transfer_ref` bypasses the need to
        // return to `bonding_curve_launchpad` by not executing the custom withdraw logic.
        let optimal_1 = coin::withdraw<CoinType>(lp, optimal_amount_1);
        let optimal_2 = primary_fungible_store::withdraw_with_ref(transfer_ref,
            signer::address_of(lp), optimal_amount_2);
        // Place the APT and FA into the liquidity pair.
        router::add_liquidity_coin<CoinType>(lp, optimal_1, optimal_2, is_stable);
    }

    //---------------------------Views---------------------------
    // Constant Product Formula
    // For higher emphasize on rewarding early adopters, this can be modified to support a sub-linear trading function.
    #[view]
    public fun get_amount_out(
        fa_reserves: u128, apt_reserves: u128, swap_to_apt: bool, amount_in: u64
    ): (u64, u64, u128, u128) {
        if (swap_to_apt) {
            let divisor = fa_reserves + (amount_in as u128);
            let apt_gained = (math128::mul_div(apt_reserves, (amount_in as u128), divisor) as u64);
            assert!(apt_gained > 0, ELIQUIDITY_PAIR_SWAP_AMOUNTOUT_INSIGNIFICANT);
            return(amount_in, apt_gained, fa_reserves + (amount_in as u128), apt_reserves
                - (apt_gained as u128))
        } else {
            let divisor = apt_reserves + (amount_in as u128);
            let fa_gained = (math128::mul_div(fa_reserves, (amount_in as u128), divisor) as u64);
            assert!(fa_gained > 0, ELIQUIDITY_PAIR_SWAP_AMOUNTOUT_INSIGNIFICANT);
            return(fa_gained, amount_in, fa_reserves - (fa_gained as u128), apt_reserves + (
                    amount_in as u128
                ))
        }
    }

    // Retrieve the frozen status of a given FA.
    #[view]
    public fun get_is_frozen_metadata(fa_object_metadata: Object<Metadata>): bool acquires LiquidityPairSmartTable {
        let liquidity_pair_smart_table =
            borrow_global<LiquidityPairSmartTable>(@resource_account);
        assert!(smart_table::contains(&liquidity_pair_smart_table.liquidity_pairs,
                fa_object_metadata),
            ELIQUIDITY_PAIR_DOES_NOT_EXIST);
        smart_table::borrow(&liquidity_pair_smart_table.liquidity_pairs, fa_object_metadata)
            .is_frozen
    }

    //---------------------------Tests---------------------------
    #[test_only]
    public fun initialize_for_test(deployer: &signer) {
        move_to(deployer, LiquidityPairSmartTable { liquidity_pairs: smart_table::new() });
    }
}
