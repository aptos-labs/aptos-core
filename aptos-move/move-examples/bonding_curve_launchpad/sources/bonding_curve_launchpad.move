module bonding_curve_launchpad::bonding_curve_launchpad {
    use std::string::{Self, String};
    use std::option;
    use std::vector;
    use aptos_framework::object::{Self, Object};
    use aptos_framework::fungible_asset::{Self, FungibleAsset, Metadata};
    use aptos_framework::primary_fungible_store;
    use aptos_framework::event;
    use aptos_framework::function_info;
    use aptos_framework::dispatchable_fungible_asset;
    use aptos_std::smart_table::{Self, SmartTable};
    use aptos_std::math128;
    use bonding_curve_launchpad::liquidity_pair;
    use bonding_curve_launchpad::resource_signer_holder;

    const INITIAL_NEW_FA_RESERVE_u64: u64 = 8_003_000_000_000;
    const INITIAL_NEW_FA_RESERVE: u128 = 8_003_000_000_000;

    /// FA name and symbol already exists on platform.
    const EFA_EXISTS_ALREADY: u64 = 10;
    /// Unknown FA. Not recognized on platform.
    const EFA_DOES_NOT_EXIST: u64 = 11;
    /// FA is globally frozen for transfers.
    const EFA_FROZEN: u64 = 13;
    /// Swap amount_in is non-postive.
    const ELIQUIDITY_PAIR_SWAP_AMOUNTIN_INVALID: u64 = 110;

    //---------------------------Events---------------------------
    #[event]
    struct FungibleAssetCreated has store, drop {
        name: String,
        symbol: String,
        max_supply: u128,
        decimals: u8,
        icon_uri: String,
        project_uri: String
    }

    //---------------------------Structs---------------------------
    struct LaunchPad has key {
        key_to_fa_controller: SmartTable<FAKey, FAController>,
    }

    struct FAKey has store, copy, drop {
        name: String,
        symbol: String,
    }

    struct FAController has key, store {
        transfer_ref: fungible_asset::TransferRef
    }

    //---------------------------Init---------------------------
    fun init_module(account: &signer) {
        move_to(account, LaunchPad { key_to_fa_controller: smart_table::new() });
    }

    //---------------------------Dispatchable Standard---------------------------
    /// Follows the Dispatchable FA standard for creating custom withdraw logic.
    /// Each created FA has transfers disabled, until the APT reserves minimum threshold on
    /// the associated liquidity pair are met. This is referred to as graduation.
    /// - `transfer_ref` will ignore this custom withdraw logic.
    /// - Since `transfer_ref` is only available to our permissioned/explicit actions, participants will initially only
    ///   have the capability to either: Hold the FA, Swap within the `bonding_curve_launchpad` context.
    public fun withdraw<T: key>(
        store: Object<T>, amount: u64, transfer_ref: &fungible_asset::TransferRef
    ): FungibleAsset {
        let metadata = fungible_asset::transfer_ref_metadata(transfer_ref);
        assert!(!liquidity_pair::get_is_frozen_metadata(metadata), EFA_FROZEN);
        fungible_asset::withdraw_with_ref(transfer_ref, store, amount)
    }

    //---------------------------Bonding Curve Launchpad (BCL)---------------------------
    /// Participants can launch new FA's and their associated liquidity pair.
    /// Optionally, the participant can immediately perform an initial swap from APT to FA.
    public entry fun create_fa_pair(
        account: &signer,
        apt_amount_in: u64,
        name: String,
        symbol: String,
        max_supply: u128,
        decimals: u8,
        icon_uri: String,
        project_uri: String
    ) acquires LaunchPad {
        // Create, mint, and control the FA from within the resource_account's `bonding_curve_launchpad`.
        let fa_key = FAKey { name, symbol };
        let fa_address = create_fa(fa_key, name, symbol, max_supply, decimals, icon_uri,
            project_uri);
        let fa_metadata_obj = object::address_to_object(fa_address);
        // `transfer_ref` is required for swapping in `liquidity_pair`. Otherwise, the custom withdraw function would
        // block the transfer of APT to the creator.
        let fa_smart_table = borrow_global<LaunchPad>(@bonding_curve_launchpad);
        let transfer_ref = &smart_table::borrow(&fa_smart_table.key_to_fa_controller, fa_key)
            .transfer_ref;
        // Create the liquidity pair between APT and the new FA. Include the initial creator swap, if needed.
        liquidity_pair::register_liquidity_pair(transfer_ref, account, fa_metadata_obj,
            apt_amount_in, max_supply);
    }

    /// Swap from FA to APT, or vice versa, through `liquidity_pair`.
    public entry fun swap(
        account: &signer,
        name: String,
        symbol: String,
        swap_to_apt: bool,
        amount_in: u64
    ) acquires LaunchPad {
        // Verify the `amount_in` is valid and that the FA exists.
        assert!(amount_in > 0, ELIQUIDITY_PAIR_SWAP_AMOUNTIN_INVALID);
        let fa_key = FAKey { name, symbol };
        let fa_smart_table = borrow_global<LaunchPad>(@bonding_curve_launchpad);
        assert!(smart_table::contains(&fa_smart_table.key_to_fa_controller, fa_key),
            EFA_DOES_NOT_EXIST);
        // `transfer_ref` is used to bypass the `is_frozen` status of the FA. Without this, the defined dispatchable
        // withdraw function would prevent the ability to transfer the participant's FA onto the liquidity pair.
        let transfer_ref = &smart_table::borrow(&fa_smart_table.key_to_fa_controller, fa_key)
            .transfer_ref;
        let fa_metadata_obj =
            object::address_to_object(get_fa_obj_address(name, symbol));
        // Initiate the swap on the associated liquidity pair.
        if (swap_to_apt) {
            liquidity_pair::internal_swap_fa_to_apt(transfer_ref, account, fa_metadata_obj,
                amount_in);
        } else {
            liquidity_pair::internal_swap_apt_to_fa(transfer_ref, account, fa_metadata_obj,
                amount_in);
        }
    }

    //---------------------------Internal---------------------------z
    fun create_fa(
        fa_key: FAKey,
        name: String,
        symbol: String,
        max_supply: u128,
        decimals: u8,
        icon_uri: String,
        project_uri: String
    ): address acquires LaunchPad {
        // Only unique entries of the FA key (name and symbol) can be launched.
        let fa_smart_table = borrow_global_mut<LaunchPad>(@bonding_curve_launchpad);
        assert!(!smart_table::contains(&fa_smart_table.key_to_fa_controller, fa_key),
            EFA_EXISTS_ALREADY);
        // The FA's name and symbol is combined, to create a seed for deterministic object creation.
        // Beneficial for retrieving the FA's object address, after the initial creation,
        // like during `get_fa_obj_address(...)`.
        let fa_key_seed = *string::bytes(&name);
        vector::append(&mut fa_key_seed, b"-");
        vector::append(&mut fa_key_seed, *string::bytes(&symbol));
        let fa_obj_constructor_ref = &object::create_named_object(&resource_signer_holder::get_signer(),
            fa_key_seed);
        // Create the FA and it's associated capabilities within `bonding_curve_launchpad`. Plus, mint the predefined
        // amount of FA to the resource_account module.
        let base_unit_max_supply = option::some(max_supply * math128::pow(10, (decimals as u128)));
        primary_fungible_store::create_primary_store_enabled_fungible_asset(
            fa_obj_constructor_ref,
            base_unit_max_supply,
            name,
            symbol,
            decimals,
            icon_uri,
            project_uri);
        let mint_ref = fungible_asset::generate_mint_ref(fa_obj_constructor_ref);
        let transfer_ref = fungible_asset::generate_transfer_ref(fa_obj_constructor_ref);
        primary_fungible_store::mint(&mint_ref, @bonding_curve_launchpad,
            INITIAL_NEW_FA_RESERVE_u64);
        // Define the dispatchable FA's withdraw as a conditionally global freezing effect.
        let permissioned_withdraw = function_info::new_function_info(&resource_signer_holder::get_signer(),
            string::utf8(b"bonding_curve_launchpad"),
            string::utf8(b"withdraw"));
        dispatchable_fungible_asset::register_dispatch_functions(fa_obj_constructor_ref,
            option::some(permissioned_withdraw),
            option::none(),
            option::none(),);
        // Store `transfer_ref` for later usage, using the FA's name and symbol as the key.
        // `tranfer_ref` will be required to allow the resource_account to ignore the dispatchable withdraw's frozen
        // status for each swap of an FA on `liquidity_pair`.
        smart_table::add(&mut fa_smart_table.key_to_fa_controller, fa_key, FAController {
                transfer_ref
            });
        event::emit(FungibleAssetCreated {
                name,
                symbol,
                max_supply: INITIAL_NEW_FA_RESERVE,
                decimals,
                icon_uri,
                project_uri
            });

        get_fa_obj_address(name, symbol)
    }

    //---------------------------Views---------------------------
    // Calculate the deterministic address of an FA using it's unique name and symbol combination.
    #[view]
    public fun get_fa_obj_address(name: String, symbol: String): address {
        let fa_key_seed = *string::bytes(&name);
        vector::append(&mut fa_key_seed, b"-");
        vector::append(&mut fa_key_seed, *string::bytes(&symbol));

        object::create_object_address(&@bonding_curve_launchpad, fa_key_seed)
    }

    // Retrieve the FA balance of a given user's address.
    #[view]
    public fun get_balance(name: String, symbol: String, user: address): u64 {
        let fa_metadata_obj: Object<Metadata> =
            object::address_to_object(get_fa_obj_address(name, symbol));

        primary_fungible_store::balance(user, fa_metadata_obj)
    }

    // Retrieve the Metadata object of a given FA's unique name and symbol.
    #[view]
    public fun get_metadata(name: String, symbol: String): Object<Metadata> {
        object::address_to_object(get_fa_obj_address(name, symbol))
    }

    // Retrieve frozen status of a given FA's unique name and symbol, from associated `liquidity_pair` state.
    #[view]
    public fun get_is_frozen(name: String, symbol: String): bool {
        let fa_metadata = get_metadata(name, symbol);
        liquidity_pair::get_is_frozen_metadata(fa_metadata)
    }

    //---------------------------Tests---------------------------
    #[test_only]
    public fun initialize_for_test(deployer: &signer) {
        move_to(deployer, LaunchPad { key_to_fa_controller: smart_table::new() });
    }
}
