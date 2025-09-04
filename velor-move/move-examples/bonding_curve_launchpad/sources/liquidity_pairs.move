module bonding_curve_launchpad::liquidity_pairs {
    use std::string::{Self, String};
    use std::vector;
    use velor_std::signer;
    use velor_std::math128;
    use velor_framework::coin;
    use velor_framework::velor_account;
    use velor_framework::velor_coin::{VelorCoin};
    use velor_framework::object::{Self, Object, ExtendRef};
    use velor_framework::event;
    use velor_framework::fungible_asset;
    use velor_framework::fungible_asset::{Metadata, TransferRef, FungibleAsset, FungibleStore};
    use velor_framework::primary_fungible_store;
    use swap::router;
    use swap::liquidity_pool;
    use swap::coin_wrapper;
    friend bonding_curve_launchpad::bonding_curve_launchpad;

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
    struct Pairs has key {
        signer_extender: ExtendRef
    }

    #[resource_group_member(group = velor_framework::object::ObjectGroup)]
    struct LiquidityPair has store, key {
        extend_ref: ExtendRef,
        is_enabled: bool,
        is_frozen: bool,
        fa_reserves: u128,
        apt_reserves: u128,
        fa_store: Object<FungibleStore>,
    }

    //---------------------------Init---------------------------
    fun init_module(account: &signer) {
        let signer_extender = object::generate_extend_ref(
            &object::create_sticky_object(@bonding_curve_launchpad)
        );
        move_to(account, Pairs { signer_extender });
    }


    //---------------------------Views---------------------------
    // Constant Product Formula
    // For higher emphasize on rewarding early adopters, this can be modified to support a sub-linear trading function.
    #[view]
    public fun get_amount_out(
        fa_reserves: u128,
        apt_reserves: u128,
        swap_to_apt: bool,
        amount_in: u64
    ): (u64, u64, u128, u128) {
        if (swap_to_apt) {
            let divisor = fa_reserves + (amount_in as u128);
            let apt_gained = (math128::mul_div(apt_reserves, (amount_in as u128), divisor) as u64);
            let fa_updated_reserves = fa_reserves + (amount_in as u128);
            let apt_updated_reserves = apt_reserves - (apt_gained as u128);
            assert!(apt_gained > 0, ELIQUIDITY_PAIR_SWAP_AMOUNTOUT_INSIGNIFICANT);
            (amount_in, apt_gained, fa_updated_reserves, apt_updated_reserves)
        } else {
            let divisor = apt_reserves + (amount_in as u128);
            let fa_gained = (math128::mul_div(fa_reserves, (amount_in as u128), divisor) as u64);
            let fa_updated_reserves = fa_reserves - (fa_gained as u128);
            let apt_updated_reserves = apt_reserves + (amount_in as u128);
            assert!(fa_gained > 0, ELIQUIDITY_PAIR_SWAP_AMOUNTOUT_INSIGNIFICANT);
            (fa_gained, amount_in, fa_updated_reserves, apt_updated_reserves)
        }
    }

    // Retrieve the frozen status of a given FA.
    #[view]
    public fun get_is_frozen_metadata(
        name: String,
        symbol: String
    ): bool acquires Pairs, LiquidityPair {
        assert_liquidity_pair_exists(name, symbol);
        borrow_global<LiquidityPair>(get_pair_obj_address(name, symbol)).is_frozen
    }

    // Retrieve the address of the given FA's name and symbol.
    #[view]
    public fun get_pair_obj_address(
        name: String,
        symbol: String
    ): address acquires Pairs {
        let pairs = borrow_global<Pairs>(@bonding_curve_launchpad);
        let fa_key_seed = *string::bytes(&name);
        vector::append(&mut fa_key_seed, b"-");
        vector::append(&mut fa_key_seed, *string::bytes(&symbol));
        object::create_object_address(
            &object::address_from_extend_ref(&pairs.signer_extender),
            fa_key_seed
        )
    }

    //---------------------------DEX-helpers---------------------------
    inline fun assert_liquidity_pair_exists(
        name: String,
        symbol: String
    ) {
        let does_already_exist = object::is_object(get_pair_obj_address(name, symbol));
        assert!(does_already_exist, ELIQUIDITY_PAIR_DOES_NOT_EXIST);
    }

    //---------------------------Liquidity Pair---------------------------
    /// Creates a unique liquidity pair between a given FA and APT.
    /// Only callable from `bonding_curve_launchpad`.
    public(friend) fun register_liquidity_pair(
        name: String,
        symbol: String,
        transfer_ref: &TransferRef,
        swapper: &signer,
        fa_object_metadata: Object<Metadata>,
        apt_amount_in: u64,
        fa_minted: FungibleAsset,
        fa_initial_liquidity: u128
    ) acquires Pairs, LiquidityPair {
        // Only allow for creation of new APT-FA pairs.
        let does_already_exist = object::is_object(get_pair_obj_address(name, symbol));
        assert!(!does_already_exist, ELIQUIDITY_PAIR_EXISTS_ALREADY);
        // Every new liquidity pair will have it's information stored within an Object. This object will also be used to
        // generator signers from, for when APT or the FA needs to be transferred to and from the liquidity pair.
        // Reserves are kept on the liquidity pair object.
        // The object is identified by the unique combination of the FA's name and symbol.
        let pairs = borrow_global<Pairs>(@bonding_curve_launchpad);
        let pairs_signer = object::generate_signer_for_extending(&pairs.signer_extender);
        let fa_key_seed = *string::bytes(&name);
        vector::append(&mut fa_key_seed, b"-");
        vector::append(&mut fa_key_seed, *string::bytes(&symbol));
        let liquidity_pair_object = object::create_named_object(&pairs_signer, fa_key_seed);
        let liquidity_pair_signer = object::generate_signer(&liquidity_pair_object);
        let liquidity_pair_extend_ref = object::generate_extend_ref(&liquidity_pair_object);
        // Store all minted FA inside the liquidity_pair struct, within a Fungible Store. This object is responsible
        // for *only* it's own reserves.
        let fa_store_obj_constructor = object::create_object(@bonding_curve_launchpad);
        let fa_store = fungible_asset::create_store(&fa_store_obj_constructor, fa_object_metadata);
        fungible_asset::deposit(fa_store, fa_minted);

        // Define and store the state of the liquidity pair as:
        // Reserves, FA store, global frozen status (`is_frozen`), and enabled trading (`is_enabled`).
        // Initial APT reserves are virtual liquidity, for less extreme initial swaps (avoiding early adopter's
        // advantage, for fairness). README covers this topic in more depth.
        move_to(
            &liquidity_pair_signer,
            LiquidityPair {
                extend_ref: liquidity_pair_extend_ref,
                is_enabled: true,
                is_frozen: true,
                fa_reserves: fa_initial_liquidity,
                apt_reserves: INITIAL_VIRTUAL_APT_LIQUIDITY,
                fa_store
            }
        );
        event::emit(
            LiquidityPairCreated {
                fa_object_metadata,
                initial_fa_reserves: fa_initial_liquidity,
                initial_apt_reserves: INITIAL_VIRTUAL_APT_LIQUIDITY
            }
        );
        // Optional initial swap given to the creator of the FA.
        if (apt_amount_in > 0) {
            swap_apt_to_fa(name, symbol, transfer_ref, swapper, fa_object_metadata, apt_amount_in);
        };
    }

    /// Facilitate swapping between a given FA to APT.
    public(friend) fun swap_fa_to_apt(
        name: String,
        symbol: String,
        transfer_ref: &TransferRef,
        swapper_account: &signer,
        fa_object_metadata: Object<Metadata>,
        amount_in: u64
    ) acquires Pairs, LiquidityPair {
        // Verify the liquidity pair exists and is enabled for trading.
        assert_liquidity_pair_exists(name, symbol);
        let liquidity_pair = borrow_global_mut<LiquidityPair>(get_pair_obj_address(name, symbol));
        assert!(liquidity_pair.is_enabled, ELIQUIDITY_PAIR_DISABLED);
        // Determine the amount received of APT, when given swapper-supplied amount_in of FA.
        let (fa_given, apt_gained, fa_updated_reserves, apt_updated_reserves) = get_amount_out(
            liquidity_pair.fa_reserves,
            liquidity_pair.apt_reserves,
            true,
            amount_in
        );
        // Verify the swapper holds the FA.
        let swapper_address = signer::address_of(swapper_account);
        let does_primary_store_exist_for_swapper = primary_fungible_store::primary_store_exists(
            swapper_address,
            fa_object_metadata
        );
        assert!(does_primary_store_exist_for_swapper, EFA_PRIMARY_STORE_DOES_NOT_EXIST);
        // Perform the swap.
        // Swapper sends FA to the liquidity pair object. The liquidity pair object sends APT to the swapper, in return.
        let liquidity_pair_signer = object::generate_signer_for_extending(&liquidity_pair.extend_ref);
        let from_swapper_store = primary_fungible_store::ensure_primary_store_exists(
            swapper_address,
            fungible_asset::transfer_ref_metadata(transfer_ref)
        );
        fungible_asset::transfer_with_ref(transfer_ref, from_swapper_store, liquidity_pair.fa_store, fa_given);
        velor_account::transfer(&liquidity_pair_signer, swapper_address, apt_gained);
        // Record state changes to the liquidity pair's reserves, and emit changes as events.
        let former_fa_reserves = liquidity_pair.fa_reserves;
        let former_apt_reserves = liquidity_pair.apt_reserves;
        liquidity_pair.fa_reserves = fa_updated_reserves;
        liquidity_pair.apt_reserves = apt_updated_reserves;
        event::emit(
            LiquidityPairReservesUpdated {
                former_fa_reserves,
                former_apt_reserves,
                new_fa_reserves: fa_updated_reserves,
                new_apt_reserves: apt_updated_reserves
            }
        );
        event::emit(
            LiquidityPairSwap {
                is_fa_else_apt: false,
                gained: (apt_gained as u128),
                swapper_address
            }
        );
    }

    /// Facilitate swapping between APT to a given FA.
    public(friend) fun swap_apt_to_fa(
        name: String,
        symbol: String,
        transfer_ref: &TransferRef,
        swapper_account: &signer,
        fa_object_metadata: Object<Metadata>,
        amount_in: u64
    ) acquires Pairs, LiquidityPair {
        // Verify the liquidity pair exists and is enabled for trading.
        assert_liquidity_pair_exists(name, symbol);
        let liquidity_pair = borrow_global_mut<LiquidityPair>(get_pair_obj_address(name, symbol));
        assert!(liquidity_pair.is_enabled, ELIQUIDITY_PAIR_DISABLED);
        // Determine the amount received of FA, when given swapper-supplied amount_in of APT.
        let (fa_gained, apt_given, fa_updated_reserves, apt_updated_reserves) = get_amount_out(
            liquidity_pair.fa_reserves,
            liquidity_pair.apt_reserves,
            false,
            amount_in
        );
        // Perform the swap.
        // Swapper sends APT to the liquidity pair object. The liquidity pair object sends FA to the swapper, in return.
        // Requires the liquidity pair object's address, which is retrieved using the stored extend_ref.
        let swapper_address = signer::address_of(swapper_account);
        let liquidity_pair_address = object::address_from_extend_ref(&liquidity_pair.extend_ref);
        let to_swapper_store = primary_fungible_store::ensure_primary_store_exists(
            swapper_address,
            fungible_asset::transfer_ref_metadata(transfer_ref)
        );
        velor_account::transfer(swapper_account, liquidity_pair_address, apt_given);
        fungible_asset::transfer_with_ref(transfer_ref, liquidity_pair.fa_store, to_swapper_store, fa_gained);
        // Record state changes to the liquidity pair's reserves, and emit changes as events.
        let former_fa_reserves = liquidity_pair.fa_reserves;
        let former_apt_reserves = liquidity_pair.apt_reserves;
        liquidity_pair.fa_reserves = fa_updated_reserves;
        liquidity_pair.apt_reserves = apt_updated_reserves;
        event::emit(
            LiquidityPairReservesUpdated {
                former_fa_reserves,
                former_apt_reserves,
                new_fa_reserves: fa_updated_reserves,
                new_apt_reserves: apt_updated_reserves
            }
        );
        event::emit(
            LiquidityPairSwap {
                is_fa_else_apt: true,
                gained: (fa_gained as u128),
                swapper_address
            }
        );
        // Check for graduation requirements. The APT reserves must be above the pre-defined
        // threshold to allow for graduation.
        if (liquidity_pair.is_enabled && apt_updated_reserves > APT_LIQUIDITY_THRESHOLD) {
            graduate(liquidity_pair, fa_object_metadata, transfer_ref, apt_updated_reserves, fa_updated_reserves);
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
        router::create_pool_coin<VelorCoin>(fa_object_metadata, false);
        let liquidity_pair_signer = object::generate_signer_for_extending(&liquidity_pair.extend_ref);
        add_liquidity_coin_entry_transfer_ref<VelorCoin>(
            transfer_ref,
            &liquidity_pair_signer,
            liquidity_pair.fa_store,
            fa_object_metadata,
            false,
            ((apt_updated_reserves - (apt_updated_reserves / 10)) as u64),
            ((fa_updated_reserves - (fa_updated_reserves / 10)) as u64),
            0,
            0
        );
        // Send liquidity provider tokens to dead address.
        let apt_coin_wrapped = coin_wrapper::get_wrapper<VelorCoin>();
        let liquidity_obj = liquidity_pool::liquidity_pool(apt_coin_wrapped, fa_object_metadata, false);
        let liquidity_pair_address = signer::address_of(&liquidity_pair_signer);
        liquidity_pool::transfer(
            &liquidity_pair_signer,
            liquidity_obj,
            @0xdead,
            primary_fungible_store::balance(liquidity_pair_address, liquidity_obj)
        );
        // Emit event informing all that the liquidity pair has graduated and which DEX it graduated to.
        event::emit(
            LiquidityPairGraduated {
                fa_object_metadata,
                dex_address: @swap
            }
        );
    }

    //---------------------------DEX-helpers---------------------------
    /// Add liquidity alternative that relies on `transfer_ref`, rather than the traditional transfer
    /// found in the swap DEX.
    fun add_liquidity_coin_entry_transfer_ref<CoinType>(
        transfer_ref: &TransferRef,
        lp: &signer,
        fa_store: Object<FungibleStore>,
        token_2: Object<Metadata>,
        is_stable: bool,
        amount_1_desired: u64,
        amount_2_desired: u64,
        amount_1_min: u64,
        amount_2_min: u64,
    ) {
        // Wrap APT into a FA. Then, determine the optimal amounts for providing liquidity to the given FA - APT pair.
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
        // Retrieve the APT and FA from the liquidity provider.
        // `transfer_ref` is used to avoid circular dependency during graduation. A normal transfer would require
        // visiting `bonding_curve_launchpad` to execute the custom withdraw logic. `transfer_ref` bypasses the need to
        // return to `bonding_curve_launchpad` by not executing the custom withdraw logic.
        let optimal_1 = coin::withdraw<CoinType>(lp, optimal_amount_1);
        let optimal_2 = fungible_asset::withdraw_with_ref(
            transfer_ref,
            fa_store,
            optimal_amount_2
        );
        // Place the APT and FA into the liquidity pair.
        router::add_liquidity_coin<CoinType>(lp, optimal_1, optimal_2, is_stable);
    }

    //---------------------------Tests---------------------------
    #[test_only]
    public fun initialize_for_test(deployer: &signer) {
        let signer_extender = object::generate_extend_ref(
            &object::create_sticky_object(@bonding_curve_launchpad)
        );
        move_to(deployer, Pairs { signer_extender });
    }
}
