module resource_account::bonding_curve_launchpad {
    use std::string;
    use std::option;
    use aptos_framework::object::{Self};
    use aptos_framework::fungible_asset::{Self, FungibleAsset, Metadata};
    use aptos_framework::primary_fungible_store;
    use aptos_framework::event;
    use aptos_framework::aptos_account;
    use aptos_std::smart_table::{Self, SmartTable};
    use aptos_std::math128;
    use aptos_std::string::{String};
    use aptos_std::signer;
    use aptos_std::object::{Object};
    //! Dispatchable FA future standard
    use aptos_framework::dispatchable_fungible_asset;
    use aptos_framework::function_info;
    // Friend
    use resource_account::resource_signer_holder;

    const EFA_EXISTS_ALREADY: u64 = 10;
    const EFA_DOES_NOT_EXIST: u64 = 11;
    const EFA_PRIMARY_STORE_DOES_NOT_EXIST: u64 = 12;
    const EFA_FROZEN: u64 = 13;
    const ELIQUIDITY_PAIR_EXISTS_ALREADY: u64 = 100;
    const ELIQUIDITY_PAIR_DOES_NOT_EXIST: u64 = 101;
    const ELIQUIDITY_PAIR_DISABLED: u64 = 102;
    const ELIQUIDITY_PAIR_SWAP_AMOUNTIN_INVALID: u64 = 110;
    const ELIQUIDITY_PAIR_SWAP_AMOUNTOUT_INSIGNIFICANT: u64 = 111;

    const INITIAL_NEW_FA_RESERVE: u128 = 803_000_000_000_000_000;
    const INITIAL_NEW_FA_RESERVE_u64: u64 = 803_000_000_000_000_000;
    const FA_DECIMALS: u8 = 8;
    const INITIAL_VIRTUAL_LIQUIDITY: u128 = 50_000_000_000;
    const APT_LIQUIDITY_THRESHOLD: u128 = 600_000_000_000;

    //---------------------------Events---------------------------
    #[event]
    struct FungibleAssetCreated has store, drop {
        name: string::String,
        symbol: string::String,
        max_supply: u128,
        decimals: u8,
        icon_uri: string::String,
        project_uri: string::String
    }
    #[event]
    struct LiquidityPairCreated has store, drop {
        fa_obj_address: address,
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
        fa_obj_address: address,
        fa_obj: Object<Metadata>,
        //! New DEX info...
    }

    //---------------------------Structs---------------------------
    struct LaunchPad has key {
        key_to_fa_data: SmartTable<FAKey, FAData>
    }
    struct FAKey has store, copy, drop {
        name: string::String,
        symbol: string::String,
    }
    struct FAData has key, store {
        controller: FAController,
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
    struct LiquidityPair has store {
        is_enabled: bool,
        is_frozen: bool,
        fa_reserves: u128,
        apt_reserves: u128,
        k_constant: u256
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
        let fa_key = FAKey { name, symbol };
        // * Create new FA and store FA owner obj on launchpad.
        let fa_address = create_fa(fa_key, name, symbol, max_supply, decimals, icon_uri, project_uri);
        let fa_metadata_obj = object::address_to_object(fa_address);
        register_liquidity_pair(account, fa_metadata_obj, fa_key, apt_initialPurchaseAmountIn, max_supply);
    }

    public entry fun swap_apt_to_fa (account: &signer, name: string::String, symbol: string::String, fa_amountIn: u64) acquires LaunchPad, LiquidityPairSmartTable {
        assert!(fa_amountIn > 0, ELIQUIDITY_PAIR_SWAP_AMOUNTIN_INVALID);
        let fa_key = FAKey { name, symbol };
        let fa_smartTable = borrow_global_mut<LaunchPad>(@resource_account);
        assert!(smart_table::contains(&mut fa_smartTable.key_to_fa_data, fa_key), EFA_DOES_NOT_EXIST);
        let fa_data = smart_table::borrow_mut(&mut fa_smartTable.key_to_fa_data, fa_key);
        let fa_metadata_obj:Object<Metadata> = object::address_to_object(fa_data.fa_obj_address);
        internal_swap_apt_to_fa(account, fa_smartTable, fa_metadata_obj, fa_key, fa_amountIn);
    }
    public entry fun swap_fa_to_apt (account: &signer, name: string::String, symbol: string::String, apt_amountIn: u64) acquires LaunchPad, LiquidityPairSmartTable {
        assert!(apt_amountIn > 0, ELIQUIDITY_PAIR_SWAP_AMOUNTIN_INVALID);
        let fa_key = FAKey {
            name,
            symbol
        };
        let fa_smartTable = borrow_global_mut<LaunchPad>(@resource_account);
        assert!(smart_table::contains(&mut fa_smartTable.key_to_fa_data, fa_key), EFA_DOES_NOT_EXIST);
        let fa_data = smart_table::borrow_mut(&mut fa_smartTable.key_to_fa_data, fa_key);
        let fa_metadata_obj:Object<Metadata> = object::address_to_object(fa_data.fa_obj_address);
        internal_swap_fa_to_apt(account, fa_smartTable, fa_metadata_obj, fa_key, apt_amountIn);
    }

    fun create_fa(
        fa_key: FAKey,
        name: string::String,
        symbol: string::String,
        max_supply: u128,
        decimals: u8,
        icon_uri: string::String,
        project_uri: string::String
    ): address acquires LaunchPad {
        let fa_smartTable = borrow_global_mut<LaunchPad>(@resource_account);
        assert!(!smart_table::contains(&mut fa_smartTable.key_to_fa_data, fa_key), EFA_EXISTS_ALREADY);

        let base_unit_max_supply: option::Option<u128> = option::some(max_supply * math128::pow(10, (decimals as u128)));
        let fa_obj_constructor_ref = &object::create_sticky_object(@resource_account); // FA object container. Need to store FA, somewhere, so object is it's home. FA obj is essentially the FA.
        // Creates FA, the primary store for the FA on the resource account defined by constructor, AND defines the max supply.
        primary_fungible_store::create_primary_store_enabled_fungible_asset(
            fa_obj_constructor_ref,
            base_unit_max_supply,
            name,
            symbol,
            decimals,
            icon_uri,
            project_uri
        );
        // Ref's held by the contract.
        let mint_ref = fungible_asset::generate_mint_ref(fa_obj_constructor_ref);
        let burn_ref = fungible_asset::generate_burn_ref(fa_obj_constructor_ref);
        let transfer_ref = fungible_asset::generate_transfer_ref(fa_obj_constructor_ref);

        //! Needs testing against Dispatchable FA standard.
        let withdraw_limitations = function_info::new_function_info(
            &resource_signer_holder::get_signer(),
            string::utf8(b"controlled_token"),
            string::utf8(b"withdraw")
        );

        let fa_obj_signer = object::generate_signer(fa_obj_constructor_ref);
        let fa_obj_address = signer::address_of(&fa_obj_signer);
        primary_fungible_store::mint(&mint_ref, @resource_account, INITIAL_NEW_FA_RESERVE_u64);

        let fa_controller = FAController {
            mint_ref,
            burn_ref,
            transfer_ref,
        };
        let fa_data = FAData {
            controller: fa_controller,
            fa_obj_address
        };
        smart_table::add(
            &mut fa_smartTable.key_to_fa_data,
            fa_key,
            fa_data
        );

        event::emit(FungibleAssetCreated {
            name: name,
            symbol: symbol,
            max_supply: INITIAL_NEW_FA_RESERVE,
            decimals: decimals,
            icon_uri: icon_uri,
            project_uri: project_uri
        });

        return fa_obj_address
    }


    //! Needs testing against Dispatchable FA standard.
    public fun withdraw<T: key>(
        store: Object<T>,
        amount: u64,
        transfer_ref: &fungible_asset::TransferRef
    ): FungibleAsset acquires LiquidityPairSmartTable {
        let metadata = fungible_asset::transfer_ref_metadata(transfer_ref);
        let liquidity_pair_smartTable = borrow_global_mut<LiquidityPairSmartTable>(@resource_account);
        assert!(smart_table::contains(&mut liquidity_pair_smartTable.liquidity_pairs, metadata), ELIQUIDITY_PAIR_DOES_NOT_EXIST);
        let liquidity_pair = smart_table::borrow_mut(&mut liquidity_pair_smartTable.liquidity_pairs, metadata);
        assert!(!liquidity_pair.is_frozen, EFA_FROZEN); // If the pair is enabled, then FA is frozen. Vice versa applies.
        fungible_asset::withdraw_with_ref(transfer_ref, store, amount)
    }


    //---------------------------Liquidity Pair---------------------------
    fun register_liquidity_pair(account: &signer, fa_metadata: Object<Metadata>, fa_key: FAKey, apt_initialPurchaseAmountIn: u64, fa_initialLiquidity: u128) acquires LaunchPad, LiquidityPairSmartTable {
        let fa_smartTable = borrow_global_mut<LaunchPad>(@resource_account);
        let fa_data = smart_table::borrow_mut(&mut fa_smartTable.key_to_fa_data, fa_key);
        let liquidity_pair_smartTable = borrow_global_mut<LiquidityPairSmartTable>(@resource_account);
        assert!(!smart_table::contains(&mut liquidity_pair_smartTable.liquidity_pairs, fa_metadata), ELIQUIDITY_PAIR_EXISTS_ALREADY);
        //* FA already exists on the platform, since it's a shared address.
        //* Initial APT reserves are virtual liquidity.
        let k_constant: u256 = (fa_initialLiquidity as u256) * (INITIAL_VIRTUAL_LIQUIDITY as u256);
        let initial_liquidity_pair = LiquidityPair {
            is_enabled: true,
            is_frozen: true,
            fa_reserves: fa_initialLiquidity,
            apt_reserves: INITIAL_VIRTUAL_LIQUIDITY,
            k_constant: k_constant
        };
        smart_table::add(
            &mut liquidity_pair_smartTable.liquidity_pairs,
            fa_metadata,
            initial_liquidity_pair
        );

        event::emit(LiquidityPairCreated {
            fa_obj_address: fa_data.fa_obj_address,
            fa_obj: fa_metadata,
            initial_fa_reserves: fa_initialLiquidity,
            initial_apt_reserves: INITIAL_VIRTUAL_LIQUIDITY,
            k: k_constant
        });

        if(apt_initialPurchaseAmountIn != 0)
            internal_swap_apt_to_fa(account, fa_smartTable, fa_metadata, fa_key, apt_initialPurchaseAmountIn);
    }

    fun internal_swap_fa_to_apt(account: &signer, fa_smartTable: &mut LaunchPad,  fa_metadata: Object<Metadata>, fa_key: FAKey, amountIn: u64) acquires LiquidityPairSmartTable {
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
        let fa_data = smart_table::borrow_mut(&mut fa_smartTable.key_to_fa_data, fa_key);
        let does_primary_store_exist_for_swapper = primary_fungible_store::primary_store_exists(swapper_address, fa_metadata);
        assert!(does_primary_store_exist_for_swapper, EFA_PRIMARY_STORE_DOES_NOT_EXIST);
        let account_address = signer::address_of(account);
        primary_fungible_store::transfer_with_ref(&fa_data.controller.transfer_ref, swapper_address, @resource_account, fa_given);
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

    fun internal_swap_apt_to_fa(account: &signer, fa_smartTable: &mut LaunchPad,  fa_metadata: Object<Metadata>, fa_key: FAKey, amountIn: u64) acquires LiquidityPairSmartTable {
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
        let fa_data = smart_table::borrow_mut(&mut fa_smartTable.key_to_fa_data, fa_key);
        let does_primary_store_exist_for_swapper = primary_fungible_store::primary_store_exists(swapper_address, fa_metadata);
        if(!does_primary_store_exist_for_swapper){
            primary_fungible_store::create_primary_store(swapper_address, fa_metadata);
        };
        aptos_account::transfer(account, @resource_account, apt_given);
        primary_fungible_store::transfer_with_ref(&fa_data.controller.transfer_ref, @resource_account, swapper_address, fa_gained);
        // Disable transfers from users.
        //? Do I really need to check this every time?
        if(!primary_fungible_store::is_frozen(swapper_address, fa_metadata))
            primary_fungible_store::set_frozen_flag(&fa_data.controller.transfer_ref, swapper_address, true);

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


        if(apt_updated_reserves > APT_LIQUIDITY_THRESHOLD){
            //! Offload onto permissionless DEX.
            //! ...Move all APT and FA reserves to the DEX.

            //! Burn any LP tokens received by the DEX.
            //! ...

            liquidity_pair.is_enabled = false;
            liquidity_pair.is_frozen = false;

            //? Destroy refs???
            //? ...

            event::emit(LiquidityPairGraduated {
                fa_obj_address: fa_data.fa_obj_address,
                fa_obj: fa_metadata,
                //! New DEX info...
            });
        }
    }

    #[view]
    public fun get_amount_out(fa_reserves: u128, apt_reserves: u128, supplied_fa_else_apt: bool, amountIn: u64): (u64, u64, u128, u128) {
        if (supplied_fa_else_apt) {
            let apt_gained: u64 = (((apt_reserves as u256)* (amountIn as u256)) / ((fa_reserves  as u256) + (amountIn as u256)) as u64);
            assert!(apt_gained > 0, ELIQUIDITY_PAIR_SWAP_AMOUNTOUT_INSIGNIFICANT);
            return (amountIn, apt_gained, fa_reserves+(amountIn as u128), apt_reserves-(apt_gained as u128))
        }
        else {
            let fa_gained: u64 = (((fa_reserves as u256) * (amountIn as u256))/((apt_reserves as u256) - (amountIn as u256)) as u64);
            assert!(fa_gained > 0, ELIQUIDITY_PAIR_SWAP_AMOUNTOUT_INSIGNIFICANT);
            return (fa_gained, amountIn, fa_reserves-(fa_gained as u128), apt_reserves+(amountIn as u128))
        }

    }
    #[view]
    public fun get_balance(name: String, symbol: String, user: address): u64 acquires LaunchPad{
        let fa_key = FAKey { name, symbol };
        let fa_smartTable = borrow_global<LaunchPad>(@resource_account);
        let fa_data = smart_table::borrow(&fa_smartTable.key_to_fa_data, fa_key);
        let fa_metadata_obj:Object<Metadata> = object::address_to_object(fa_data.fa_obj_address);

        primary_fungible_store::balance(user, fa_metadata_obj)
    }


    //---------------------------Tests---------------------------
    #[test(account = @resource_account, sender = @memecoin_creator_addr)]
    fun test_create_fa(account: &signer, sender: &signer) acquires LaunchPad, LiquidityPairSmartTable {
        init_module(account);

        create_fa_pair(
            sender,
            0,
            string::utf8(b"SheepyCoin"),
            string::utf8(b"SHEEP"),
            INITIAL_NEW_FA_RESERVE,
            FA_DECIMALS,
            string::utf8(b"https://t4.ftcdn.net/jpg/03/12/95/13/360_F_312951336_8LxW7gBLHslTnpbOAwxFo5FpD2R5vGxu.jpg"),
            string::utf8(b"https://t4.ftcdn.net/jpg/03/12/95/13/360_F_312951336_8LxW7gBLHslTnpbOAwxFo5FpD2R5vGxu.jpg")
        );
        create_fa_pair(
            sender,
            0,
            string::utf8(b"PoggieCoin"),
            string::utf8(b"POGGIE"),
            INITIAL_NEW_FA_RESERVE,
            FA_DECIMALS,
            string::utf8(b"https://t4.ftcdn.net/jpg/03/12/95/13/360_F_312951336_8LxW7gBLHslTnpbOAwxFo5FpD2R5vGxu.jpg"),
            string::utf8(b"https://t4.ftcdn.net/jpg/03/12/95/13/360_F_312951336_8LxW7gBLHslTnpbOAwxFo5FpD2R5vGxu.jpg")
        );
    }

}
