module bonding_curve_launchpad_addr::bonding_curve_launchpad {
    use std::string;
    use std::option;
    use aptos_framework::object::{Self};
    use aptos_framework::fungible_asset::{Self, Metadata};
    use aptos_framework::primary_fungible_store;
    use aptos_std::smart_table::{Self, SmartTable};
    use aptos_std::math128;
    use aptos_std::string::{String, utf8, bytes};
    use aptos_std::signer;
    use aptos_std::aptos_account;
    use aptos_std::object::{Object};
    // DEBUG
    use std::debug;

    const EFA_EXISTS_ALREADY: u64 = 10;
    const ELIQUIDITY_PAIR_EXISTS_ALREADY: u64 = 100;
    const ELIQUIDITY_PAIR_DOES_NOT_EXIST: u64 = 101;
    const ELIQUIDITY_PAIR_DISABLED: u64 = 102;

    const INITIAL_NEW_FA_RESERVE: u128 = 803_000_000_000_000_000;
    const FA_DECIMALS: u8 = 8;
    const INITIAL_VIRTUAL_LIQUIDITY: u128 = 50_000_000_000;
    const APT_LIQUIDITY_THRESHOLD: u128 = 600_000_000_000;

    struct LaunchPad has key {
        key_to_fa_data: SmartTable<FAKey, FAData>,
    }
    struct FAKey has store, copy, drop {
        name: string::String,
        symbol: string::String,
    }
    struct FAData has key, store {
        controller: FAController,
        description: FADescription,
        fa_obj_address: address, // Since we generate using sticky invocation, we can't reference using a symbol. Instead, just keep a mapping to the final address.
    }
    struct FAController has key, store {
        mint_ref: fungible_asset::MintRef,
        burn_ref: fungible_asset::BurnRef,
        transfer_ref: fungible_asset::TransferRef
    }

    struct LiquidityPairSmartTable has key {
        liquidity_pairs: SmartTable<Object<Metadata>, LiquidityPair>
    }
    // contains data for pair, functions will (retrieve and) modify this data
    struct LiquidityPair has store {
        enabled: bool,
        fa_reserves: u128,
        apt_reserves: u128,
        k_constant: u256
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

    //---------------------------Init---------------------------
    fun init_module(account: &signer) {
        let fa_smartTable: LaunchPad = LaunchPad {
            key_to_fa_data: smart_table::new()
        };
        let liquidity_pair_table: LiquidityPairSmartTable = LiquidityPairSmartTable {
            liquidity_pairs: smart_table::new()
        };

        move_to(account, fa_smartTable);
        move_to(account, liquidity_pair_table);
    }


    //---------------------------Bonding Curve Launchpad (BCL)---------------------------
    public entry fun create_fa_pair(
        account: &signer,
        apt_initialPurchaseAmountIn: u64,
        name: string::String,
        symbol: string::String,
        max_supply: u128,
        decimals: u8,
        icon_uri: string::String,
        project_uri: string::String
    ) acquires LaunchPad, LiquidityPairSmartTable {
        let fa_key = FAKey {
            name,
            symbol
        };
        let fa_description = FADescription {
                name: name,
                symbol: symbol,
                max_supply: max_supply,
                decimals: decimals,
                icon_uri: icon_uri,
                project_uri: project_uri
        };
        // * Create new FA and store FA owner obj on launchpad.
        let fa_address = create_fa(account, fa_key, fa_description);
        let fa_metadata_obj = object::address_to_object(fa_address);
        register_liquidity_pair(account, fa_metadata_obj, fa_key, apt_initialPurchaseAmountIn, max_supply);
    }

    public entry fun buy_fa () {
        // ! Can transfer this directly to the account buyer.
    }

    fun create_fa(
        account: &signer,
        fa_key: FAKey,
        fa_description: FADescription
    ): address acquires LaunchPad {
        let fa_smartTable = borrow_global_mut<LaunchPad>(@bonding_curve_launchpad_addr);
        assert!(!smart_table::contains(&mut fa_smartTable.key_to_fa_data, fa_key), EFA_EXISTS_ALREADY);

        let base_unit_max_supply: option::Option<u128> = option::some(fa_description.max_supply * math128::pow(10, (fa_description.decimals as u128)));
        let fa_obj_constructor_ref = &object::create_sticky_object(@bonding_curve_launchpad_addr); // FA object container. Need to store FA, somewhere, so object is it's home. FA obj is essentially the FA.
        // Creates FA, the primary store for the FA on the resource account defined by constructor, AND mints the initial supply.
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
        let fa_obj_signer = object::generate_signer(fa_obj_constructor_ref);
        let fa_obj_address = signer::address_of(&fa_obj_signer);

        let fa_data = FAData {
            controller: fa_controller,
            description: fa_description,
            fa_obj_address
        };
        smart_table::add(
            &mut fa_smartTable.key_to_fa_data,
            fa_key,
            fa_data
        );

        return fa_obj_address
    }



    //---------------------------Liquidity Pair---------------------------
    //! Temporarily, x*y=k.
    // Only callable from bonding_curve_launchpad Module. By the time we reach this, we'll already have the FAData.
    fun register_liquidity_pair(account: &signer, fa_metadata: Object<Metadata>, fa_key: FAKey, apt_initialPurchaseAmountIn: u64, fa_initialLiquidity: u128) acquires LaunchPad, LiquidityPairSmartTable {
        let fa_smartTable = borrow_global_mut<LaunchPad>(@bonding_curve_launchpad_addr);
        let fa_data = smart_table::borrow_mut(&mut fa_smartTable.key_to_fa_data, fa_key);
        let liquidity_pair_smartTable = borrow_global_mut<LiquidityPairSmartTable>(@bonding_curve_launchpad_addr);
        assert!(!smart_table::contains(&mut liquidity_pair_smartTable.liquidity_pairs, fa_metadata), ELIQUIDITY_PAIR_EXISTS_ALREADY);
        //* FA already exists on the platform, since it's a shared address.
        //* Initial APT reserves are virtual liquidity.
        let k_constant: u256 = (fa_initialLiquidity as u256) * (INITIAL_VIRTUAL_LIQUIDITY as u256);
        let initial_liquidity_pair = LiquidityPair {
            enabled: true,
            fa_reserves: fa_initialLiquidity,
            apt_reserves: INITIAL_VIRTUAL_LIQUIDITY,
            k_constant: k_constant
        };
        smart_table::add(
            &mut liquidity_pair_smartTable.liquidity_pairs,
            fa_metadata,
            initial_liquidity_pair
        );

        // if(apt_initialPurchaseAmountIn != 0)
        //     swap_apt_to_fa(account, fa_metadata, fa_data.controller.transfer_ref, apt_initialPurchaseAmountIn);

        //! Event for liquidity pair created...
    }

    public(friend) fun swap_fa_to_apt(fa_metadata: Object<Metadata>, amountIn: u64) acquires LiquidityPairSmartTable {
        let liquidity_pair_smartTable = borrow_global_mut<LiquidityPairSmartTable>(@bonding_curve_launchpad_addr);
        assert!(smart_table::contains(&mut liquidity_pair_smartTable.liquidity_pairs, fa_metadata), ELIQUIDITY_PAIR_DOES_NOT_EXIST);
        let liquidity_pair = smart_table::borrow_mut(&mut liquidity_pair_smartTable.liquidity_pairs, fa_metadata);
        assert!(!liquidity_pair.enabled, ELIQUIDITY_PAIR_DISABLED);

    }

    public(friend) fun swap_apt_to_fa(account: &signer, fa_metadata: Object<Metadata>, fa_transfer_ref: fungible_asset::TransferRef, amountIn: u64) acquires LiquidityPairSmartTable {
        let liquidity_pair_smartTable = borrow_global_mut<LiquidityPairSmartTable>(@bonding_curve_launchpad_addr);
        assert!(smart_table::contains(&mut liquidity_pair_smartTable.liquidity_pairs, fa_metadata), ELIQUIDITY_PAIR_DOES_NOT_EXIST);
        let liquidity_pair = smart_table::borrow_mut(&mut liquidity_pair_smartTable.liquidity_pairs, fa_metadata);
        assert!(!liquidity_pair.enabled, ELIQUIDITY_PAIR_DISABLED);

        //* amountIn might end up changing here, so we'll use the value returned.
        let swapper_address = signer::address_of(account);
        let (fa_gained, apt_given, fa_updated_reserves, apt_updated_reserves) = get_amount_out(
            liquidity_pair.fa_reserves,
            liquidity_pair.apt_reserves,
            false,
            amountIn
        );
        aptos_account::transfer(account, @bonding_curve_launchpad_addr, apt_given);
        primary_fungible_store::transfer_with_ref(&fa_transfer_ref, @bonding_curve_launchpad_addr, swapper_address, fa_gained);

        // Disable transfers from users.
        //? Do I really need to check this every time?
        if(!primary_fungible_store::is_frozen(swapper_address, fa_metadata))
            primary_fungible_store::set_frozen_flag(&fa_transfer_ref, swapper_address, true);

        liquidity_pair.fa_reserves = fa_updated_reserves;
        liquidity_pair.apt_reserves = apt_updated_reserves;

        //! Event for liquidity pair updated...

        if(apt_updated_reserves > APT_LIQUIDITY_THRESHOLD){
            //! Offload onto permissionless DEX.
            //! ...

            //! Event for liquidity pair offloaded...
        }
    }



    //**
    //* fa_gained, apt_given, token0UpdatedReserves, token1UpdatedReserves
    //* postive/negative relates to the pool, not the user swapping.
    #[view]
    public fun get_amount_out(fa_reserves: u128, apt_reserves: u128, supplied_fa_else_apt: bool, amountIn: u64): (u64, u64, u128, u128) {
        if (supplied_fa_else_apt) {
            let apt_gained: u64 = (((apt_reserves as u256)* (amountIn as u256)) / ((fa_reserves  as u256) + (amountIn as u256)) as u64);
            return (amountIn, apt_gained, fa_reserves+(amountIn as u128), apt_reserves-(apt_gained as u128))
        }
        else {
            let fa_gained: u64 = (((fa_reserves as u256) * (amountIn as u256))/((apt_reserves as u256)- (amountIn as u256)) as u64);
            return (fa_gained, amountIn, fa_reserves-(fa_gained as u128), apt_reserves+(amountIn as u128))
        }

    }

    // #[view]
    // public fun get_reserves(liquidity_pair: &LiquidityPair){
    //     return
    // }


    //---------------------------Tests---------------------------


    #[test(account = @bonding_curve_launchpad_addr, sender = @memecoin_creator_addr)]
    fun test_create_fa(account: &signer, sender: &signer) acquires LaunchPad, LiquidityPairSmartTable {
        init_module(account);
        create_fa_pair(
            sender,
            100_000_000,
            string::utf8(b"SheepyCoin"),
            string::utf8(b"SHEEP"),
            INITIAL_NEW_FA_RESERVE,
            FA_DECIMALS,
            string::utf8(b"https://t4.ftcdn.net/jpg/03/12/95/13/360_F_312951336_8LxW7gBLHslTnpbOAwxFo5FpD2R5vGxu.jpg"),
            string::utf8(b"https://t4.ftcdn.net/jpg/03/12/95/13/360_F_312951336_8LxW7gBLHslTnpbOAwxFo5FpD2R5vGxu.jpg")
        );
        create_fa_pair(
            sender,
            100_000_000,
            string::utf8(b"PoggieCoin"),
            string::utf8(b"POGGIE"),
            INITIAL_NEW_FA_RESERVE,
            FA_DECIMALS,
            string::utf8(b"https://t4.ftcdn.net/jpg/03/12/95/13/360_F_312951336_8LxW7gBLHslTnpbOAwxFo5FpD2R5vGxu.jpg"),
            string::utf8(b"https://t4.ftcdn.net/jpg/03/12/95/13/360_F_312951336_8LxW7gBLHslTnpbOAwxFo5FpD2R5vGxu.jpg")
        );
        debug::print(&utf8(b"PoggieWoggies"));
    }



}
