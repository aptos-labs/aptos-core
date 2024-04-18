module bonding_curve_launchpad_addr::bonding_curve_launchpad {
    use std::string;
    use std::option;
    use aptos_framework::fungible_asset;
    use aptos_framework::primary_fungible_store;
    use aptos_framework::object;
    use aptos_std::smart_table::{Self, SmartTable};
    use aptos_std::math128;


    struct LaunchPad has key {
        description_to_fa: SmartTable<FADescription, FAController>,
    }

    struct FAController has key, store {
        mint_ref: fungible_asset::MintRef,
        burn_ref: fungible_asset::BurnRef,
        transfer_ref: fungible_asset::TransferRef
    }

    struct FADescription has store, copy, drop {
        name: string::String,
        symbol: string::String,
        max_supply: u128,
        decimals: u8,
        icon_uri: string::String,
        project_uri: string::String
    }

    fun init_module(account: &signer) {
        let fa_smartTable: LaunchPad = LaunchPad {
            description_to_fa: smart_table::new()
        };
        move_to(account, fa_smartTable);
    }


    public entry fun create_fa_pair(
        account: &signer,
        name: string::String,
        symbol: string::String,
        max_supply: u128,
        decimals: u8,
        icon_uri: string::String,
        project_uri: string::String
    ) acquires LaunchPad {
        let fa_description = FADescription {
                name: name,
                symbol: symbol,
                max_supply: max_supply,
                decimals: decimals,
                icon_uri: icon_uri,
                project_uri: project_uri
        };
        // * Create new FA and store obj on launchpad. Need to disable transfer until threshold is met.
        create_fa(fa_description);

        // ! Create new pair between APT and new FA.
        // ...
    }

    public entry fun buy_fa () {
        // ! Can transfer this directly to the account buyer.
    }

    fun create_fa(
        fa_description: FADescription
    ) acquires LaunchPad {
        let fa_smartTable = borrow_global_mut<LaunchPad>(@bonding_curve_launchpad_addr);

        // ! Verify it doesn't already exist in smart table.
        // ! ...

        let base_unit_max_supply: option::Option<u128> = option::some(fa_description.max_supply * math128::pow(10, (fa_description.decimals as u128)));

        // FA object container. Need to store FA, somewhere, so object is it's home. FA obj is essentially the FA.
        let fa_obj_constructor_ref = &object::create_sticky_object(@bonding_curve_launchpad_addr); // Unique object based on TX hash. So each call to this will generate a brand new obj.
        primary_fungible_store::create_primary_store_enabled_fungible_asset(
            fa_obj_constructor_ref,
            base_unit_max_supply,
            fa_description.name,
            fa_description.symbol,
            fa_description.decimals,
            fa_description.icon_uri,
            fa_description.project_uri
        );

        // Ref's held by the contract.
        let mint_ref = fungible_asset::generate_mint_ref(fa_obj_constructor_ref);
        let burn_ref = fungible_asset::generate_burn_ref(fa_obj_constructor_ref);
        let transfer_ref = fungible_asset::generate_transfer_ref(fa_obj_constructor_ref);
        smart_table::add(&mut fa_smartTable.description_to_fa, fa_description, FAController {
            mint_ref,
            burn_ref,
            transfer_ref,
        })





    }

    fun mint_fa() {

    }

    #[test(account = @bonding_curve_launchpad_addr, sender = @memecoin_creator_addr)]
    fun test_create_fa(account: &signer, sender: &signer) acquires LaunchPad {
        init_module(account);
        create_fa_pair(
            sender,
            string::utf8(b"DerzanskiCoin"),
            string::utf8(b"DERZ"),
            10000,
            8,
            string::utf8(b"https://t4.ftcdn.net/jpg/03/12/95/13/360_F_312951336_8LxW7gBLHslTnpbOAwxFo5FpD2R5vGxu.jpg"),
            string::utf8(b"https://t4.ftcdn.net/jpg/03/12/95/13/360_F_312951336_8LxW7gBLHslTnpbOAwxFo5FpD2R5vGxu.jpg")
        );
    }



}
