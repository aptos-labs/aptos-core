module bonding_curve_launchpad_addr::bonding_curve_launchpad {
    use std::string;
    use std::option;
    use aptos_framework::fungible_asset;
    use aptos_framework::primary_fungible_store;
    use aptos_framework::object;
    use aptos_std::smart_table::{Self, SmartTable};
    use aptos_std::math128;

    const EFA_EXISTS_ALREADY: u64 = 10;

    struct LaunchPad has key {
        description_to_fa: SmartTable<FAKey, FAData>,
    }


    struct FAKey has store, copy, drop {
        name: string::String,
        symbol: string::String,
    }

    struct FAData has key, store {
        controller: FAController,
        description: FADescription,
    }

    struct FAController has key, store {
        mint_ref: fungible_asset::MintRef,
        burn_ref: fungible_asset::BurnRef,
        transfer_ref: fungible_asset::TransferRef
    }

    // ! Since this data is in our primary store, do I even need this?
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
        let fa_key = FAKey {
            name: name,
            symbol: symbol,
        };
        let fa_description = FADescription {
                name: name,
                symbol: symbol,
                max_supply: max_supply,
                decimals: decimals,
                icon_uri: icon_uri,
                project_uri: project_uri
        };
        // * Create new FA and store obj on launchpad. Need to disable transfer until threshold is met.
        create_fa(fa_key, fa_description);

        // ! Create new pair between APT and new FA.
        // ...

        // ! Disable transfers for all, besides the creator (this contract).
    }

    public entry fun buy_fa () {
        // ! Can transfer this directly to the account buyer.
    }

    fun create_fa(
        fa_key: FAKey,
        fa_description: FADescription
    ) acquires LaunchPad {
        let fa_smartTable = borrow_global_mut<LaunchPad>(@bonding_curve_launchpad_addr);
        // Only one of FA of that exact description must exist.
        // ! I should I limit it down to name and symbol? Rather than including icon_uri and stuffs. Probably...
        assert!(!smart_table::contains(&mut fa_smartTable.description_to_fa, fa_key), EFA_EXISTS_ALREADY);

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
        let fa_controller = FAController {
            mint_ref,
            burn_ref,
            transfer_ref,
        };
        smart_table::add(
            &mut fa_smartTable.description_to_fa,
            fa_key,
            FAData {
                controller: fa_controller,
                description: fa_description
            }
        );
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
