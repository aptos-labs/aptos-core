module resource_account::bonding_curve_launchpad {
    use std::string::{Self, String};
    use std::option;
    use std::vector;
    use aptos_framework::object::{Self};
    use aptos_framework::fungible_asset::{Self, FungibleAsset, Metadata};
    use aptos_framework::primary_fungible_store;
    use aptos_framework::event;
    use aptos_framework::function_info;
    use aptos_framework::dispatchable_fungible_asset;
    use aptos_std::smart_table::{Self, SmartTable};
    use aptos_std::math128;
    use aptos_std::object::{Object};
    use resource_account::liquidity_pair;
    use resource_account::resource_signer_holder;

    const INITIAL_NEW_FA_RESERVE_u64: u64 = 8_003_000_000_000;
    const INITIAL_NEW_FA_RESERVE: u128 = 8_003_000_000_000;

    const EFA_EXISTS_ALREADY: u64 = 10;
    const EFA_DOES_NOT_EXIST: u64 = 11;
    const EFA_FROZEN: u64 = 13;
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
        mint_ref: fungible_asset::MintRef,
        burn_ref: fungible_asset::BurnRef,
        transfer_ref: fungible_asset::TransferRef
    }

    //---------------------------Init---------------------------
    fun init_module(account: &signer) {
        let fa_smart_table = LaunchPad {
            key_to_fa_controller: smart_table::new()
        };
        move_to(account, fa_smart_table);
    }

    //---------------------------Dispatchable Standard---------------------------
    //* FA is restricted in transfers, until an APT reserves threshold is met.
    //* - Since transfer_ref is only available to our permissioned actions, this can't be used by bad actors.
    public fun withdraw<T: key>(
        store: Object<T>,
        amount: u64,
        transfer_ref: &fungible_asset::TransferRef
    ): FungibleAsset {
        let metadata = fungible_asset::transfer_ref_metadata(transfer_ref);
        assert!(!liquidity_pair::get_is_frozen_metadata(metadata), EFA_FROZEN);
        fungible_asset::withdraw_with_ref(transfer_ref, store, amount)
    }

    //---------------------------Bonding Curve Launchpad (BCL)---------------------------
    // * Creates new FA and store FA owner obj on launchpad.
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
        let fa_key = FAKey { name, symbol };
        let fa_address = create_fa(fa_key, name, symbol, max_supply, decimals, icon_uri, project_uri);
        let fa_metadata_obj = object::address_to_object(fa_address);
        let fa_smart_table = borrow_global<LaunchPad>(@resource_account);
        let transfer_ref = &smart_table::borrow(&fa_smart_table.key_to_fa_controller, fa_key).transfer_ref;
        liquidity_pair::register_liquidity_pair(transfer_ref, account, fa_metadata_obj, apt_amount_in, max_supply);
    }

    public entry fun swap_apt_to_fa (account: &signer, name: String, symbol: String, fa_amountIn: u64) acquires LaunchPad {
        assert!(fa_amountIn > 0, ELIQUIDITY_PAIR_SWAP_AMOUNTIN_INVALID);
        let fa_key = FAKey { name, symbol };
        let fa_smart_table = borrow_global<LaunchPad>(@resource_account);
        assert!(smart_table::contains(&fa_smart_table.key_to_fa_controller, fa_key), EFA_DOES_NOT_EXIST);
        let transfer_ref = &smart_table::borrow(&fa_smart_table.key_to_fa_controller, fa_key).transfer_ref;
        let fa_metadata_obj:Object<Metadata> = object::address_to_object(get_fa_obj_address(name, symbol));

        liquidity_pair::internal_swap_apt_to_fa(transfer_ref, account, fa_metadata_obj, fa_amountIn);
    }
    public entry fun swap_fa_to_apt (account: &signer, name: String, symbol: String, apt_amountIn: u64) acquires LaunchPad {
        assert!(apt_amountIn > 0, ELIQUIDITY_PAIR_SWAP_AMOUNTIN_INVALID);
        let fa_key = FAKey { name, symbol };
        let fa_smart_table = borrow_global<LaunchPad>(@resource_account);
        assert!(smart_table::contains(&fa_smart_table.key_to_fa_controller, fa_key), EFA_DOES_NOT_EXIST);
        let transfer_ref = &smart_table::borrow(&fa_smart_table.key_to_fa_controller, fa_key).transfer_ref;
        let fa_metadata_obj:Object<Metadata> = object::address_to_object(get_fa_obj_address(name, symbol));

        liquidity_pair::internal_swap_fa_to_apt(transfer_ref, account, fa_metadata_obj, apt_amountIn);
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
        let fa_smart_table = borrow_global_mut<LaunchPad>(@resource_account);
        assert!(!smart_table::contains(&fa_smart_table.key_to_fa_controller, fa_key), EFA_EXISTS_ALREADY);
        let base_unit_max_supply: option::Option<u128> = option::some(max_supply * math128::pow(10, (decimals as u128)));
        let fa_key_seed: vector<u8> = *string::bytes(&name);
        vector::append(&mut fa_key_seed, b"-");
        vector::append(&mut fa_key_seed, *string::bytes(&symbol));
        let fa_obj_constructor_ref = &object::create_named_object(&resource_signer_holder::get_signer(), fa_key_seed);
        primary_fungible_store::create_primary_store_enabled_fungible_asset(
            fa_obj_constructor_ref,
            base_unit_max_supply,
            name,
            symbol,
            decimals,
            icon_uri,
            project_uri
        );
        let mint_ref = fungible_asset::generate_mint_ref(fa_obj_constructor_ref);
        let burn_ref = fungible_asset::generate_burn_ref(fa_obj_constructor_ref);
        let transfer_ref = fungible_asset::generate_transfer_ref(fa_obj_constructor_ref);
        primary_fungible_store::mint(&mint_ref, @resource_account, INITIAL_NEW_FA_RESERVE_u64);
        // Dispatchable FA
        let permissioned_withdraw = function_info::new_function_info(
            &resource_signer_holder::get_signer(),
            string::utf8(b"bonding_curve_launchpad"),
            string::utf8(b"withdraw")
        );
        dispatchable_fungible_asset::register_dispatch_functions(
            fa_obj_constructor_ref,
            option::some(permissioned_withdraw),
            option::none(),
            option::none(),
        );

        let fa_controller = FAController {
            mint_ref,
            burn_ref,
            transfer_ref,
        };
        smart_table::add(
            &mut fa_smart_table.key_to_fa_controller,
            fa_key,
            fa_controller
        );

        event::emit(FungibleAssetCreated {
            name: name,
            symbol: symbol,
            max_supply: INITIAL_NEW_FA_RESERVE,
            decimals: decimals,
            icon_uri: icon_uri,
            project_uri: project_uri
        });

        get_fa_obj_address(name, symbol)
    }

    //---------------------------Views---------------------------
    #[view]
    public fun get_fa_obj_address(name: String, symbol: String): address {
        let fa_key_seed: vector<u8> = *string::bytes(&name);
        vector::append(&mut fa_key_seed, b"-");
        vector::append(&mut fa_key_seed, *string::bytes(&symbol));
        let fa_obj_address = object::create_object_address(&@resource_account, fa_key_seed);

        fa_obj_address
    }
    #[view]
    public fun get_balance(name: String, symbol: String, user: address): u64 {
        let fa_metadata_obj:Object<Metadata> = object::address_to_object(get_fa_obj_address(name, symbol));

        primary_fungible_store::balance(user, fa_metadata_obj)
    }
    #[view]
    public fun get_metadata(name: String, symbol: String): Object<Metadata> {
        let fa_metadata_obj:Object<Metadata> = object::address_to_object(get_fa_obj_address(name, symbol));

        fa_metadata_obj
    }
    #[view]
    public fun get_is_frozen(name: String, symbol: String): bool {
        let fa_metadata = get_metadata(name, symbol);
        liquidity_pair::get_is_frozen_metadata(fa_metadata)
    }

    //---------------------------Tests---------------------------
    #[test_only]
    public fun initialize_for_test(deployer: &signer){
        move_to(deployer, LaunchPad {
            key_to_fa_controller: smart_table::new()
        });
    }
}
