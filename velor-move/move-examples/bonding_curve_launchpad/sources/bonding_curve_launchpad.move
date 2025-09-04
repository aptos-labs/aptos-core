module bonding_curve_launchpad::bonding_curve_launchpad {
    use std::string::{Self, String};
    use std::option;
    use std::vector;
    use velor_framework::object::{Self, Object, ExtendRef};
    use velor_framework::fungible_asset::{Self, FungibleAsset, Metadata, TransferRef};
    use velor_framework::primary_fungible_store;
    use velor_framework::event;
    use velor_framework::function_info::{Self, FunctionInfo};
    use velor_framework::dispatchable_fungible_asset;
    use velor_std::math128;
    use bonding_curve_launchpad::liquidity_pairs;

    /// FA's name and symbol already exist on the launchpad.
    const EFA_EXISTS_ALREADY: u64 = 10;
    /// Unknown FA. Not recognized on platform.
    const EFA_DOES_NOT_EXIST: u64 = 11;
    /// FA is globally frozen for transfers.
    const EFA_FROZEN: u64 = 13;
    /// Swap amount_in is non-positive.
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
        permissioned_withdraw: FunctionInfo,
        fa_generator_extend_ref: ExtendRef
    }

    #[resource_group_member(group = velor_framework::object::ObjectGroup)]
    struct FAController has key, store {
        transfer_ref: TransferRef
    }

    //---------------------------Init---------------------------
    fun init_module(account: &signer) {
        // Create and store the permissioned_withdraw function (global freezing based on FA status) while access to
        // the account signer is available.
        let permissioned_withdraw = function_info::new_function_info(
            account,
            string::utf8(b"bonding_curve_launchpad"),
            string::utf8(b"withdraw")
        );
        // Since the account signer can't be placed in storage, we'll use an object and it's retrievable signer for any
        // required signer usage. Primarily, when generating new Fungible Assets.
        let fa_generator_extend_ref = object::generate_extend_ref(
            &object::create_named_object(account, b"FA Generator")
        );
        move_to(
            account,
            LaunchPad {
                permissioned_withdraw,
                fa_generator_extend_ref
            }
        );
    }

    //---------------------------Views---------------------------
    // Calculate the deterministic address of an FA using it's unique name and symbol combination within the
    // "FA Generator" object.
    #[view]
    public fun get_fa_obj_address(
        name: String,
        symbol: String
    ): address acquires LaunchPad {
        let launchpad = borrow_global<LaunchPad>(@bonding_curve_launchpad);
        let fa_generator_address = object::address_from_extend_ref(&launchpad.fa_generator_extend_ref);
        let fa_key_seed = *string::bytes(&name);
        vector::append(&mut fa_key_seed, b"-");
        vector::append(&mut fa_key_seed, *string::bytes(&symbol));
        object::create_object_address(&fa_generator_address, fa_key_seed)
    }

    // Retrieve the FA balance of a given user's address.
    #[view]
    public fun get_balance(
        name: String,
        symbol: String,
        user: address
    ): u64 acquires LaunchPad {
        let fa_metadata_obj: Object<Metadata> = object::address_to_object(get_fa_obj_address(name, symbol));
        primary_fungible_store::balance(user, fa_metadata_obj)
    }

    // Retrieve the Metadata object of a given FA's unique name and symbol.
    #[view]
    public fun get_metadata(
        name: String,
        symbol: String
    ): Object<Metadata> acquires LaunchPad {
        object::address_to_object(get_fa_obj_address(name, symbol))
    }

    // Retrieve frozen status of a given FA's unique name and symbol, from associated `liquidity_pair` state.
    #[view]
    public fun get_is_frozen(
        name: String,
        symbol: String
    ): bool {
        liquidity_pairs::get_is_frozen_metadata(name, symbol)
    }

    //---------------------------Dispatchable Standard---------------------------
    /// Follows the Dispatchable FA standard for creating custom withdraw logic.
    /// Each created FA has transfers disabled, until the APT reserves minimum threshold on
    /// the associated liquidity pair are met. This is referred to as graduation.
    /// - `transfer_ref` will ignore this custom withdraw logic.
    /// - Since `transfer_ref` is only available to our permissioned/explicit actions, participants will initially only
    ///   have the capability to either: Hold the FA, Swap within the `bonding_curve_launchpad` context.
    public fun withdraw<T: key>(
        store: Object<T>, amount: u64,
        transfer_ref: &TransferRef
    ): FungibleAsset {
        let metadata = fungible_asset::transfer_ref_metadata(transfer_ref);
        let name = fungible_asset::name(metadata);
        let symbol = fungible_asset::symbol(metadata);
        assert!(!liquidity_pairs::get_is_frozen_metadata(name, symbol), EFA_FROZEN);
        fungible_asset::withdraw_with_ref(transfer_ref, store, amount)
    }

    //---------------------------Bonding Curve Launchpad (BCL)---------------------------
    /// Participants can launch new FA's and their associated liquidity pair.
    /// Optionally, the participant can immediately perform an initial swap from APT to FA.
    public entry fun create_fa_pair(
        creator: &signer,
        apt_amount_in: u64,
        name: String,
        symbol: String,
        max_supply: u128,
        decimals: u8,
        icon_uri: String,
        project_uri: String
    ) acquires LaunchPad, FAController {
        // Create, mint, and control the FA using the signer obtained from the "FA Generator" object.
        let (fa_address, fa_minted) = create_fa(name, symbol, max_supply, decimals, icon_uri, project_uri);
        let fa_metadata_obj = object::address_to_object(fa_address);
        // `transfer_ref` is required for swapping in `liquidity_pair`. Otherwise, the custom withdraw function would
        // block the transfer of APT to the creator.
        let transfer_ref = &borrow_global<FAController>(fa_address).transfer_ref;
        // Create the liquidity pair between APT and the new FA. Include the initial creator swap, if needed.
        liquidity_pairs::register_liquidity_pair(
            name,
            symbol,
            transfer_ref,
            creator,
            fa_metadata_obj,
            apt_amount_in,
            fa_minted,
            max_supply
        );
    }

    /// Swap from FA to APT, or vice versa, through `liquidity_pair`.
    public entry fun swap(
        account: &signer,
        name: String,
        symbol: String,
        swap_to_apt: bool,
        amount_in: u64
    ) acquires LaunchPad, FAController {
        // Verify the `amount_in` is valid and that the FA exists.
        assert!(amount_in > 0, ELIQUIDITY_PAIR_SWAP_AMOUNTIN_INVALID);
        // FA Object<Metadata> required for primary_fungible_store interactions.
        // `transfer_ref` is used to bypass the `is_frozen` status of the FA. Without this, the defined dispatchable
        // withdraw function would prevent the ability to transfer the participant's FA onto the liquidity pair.
        let fa_metadata_obj = object::address_to_object(get_fa_obj_address(name, symbol));
        let transfer_ref = &borrow_global<FAController>(get_fa_obj_address(name, symbol)).transfer_ref;
        // Initiate the swap on the associated liquidity pair.
        if (swap_to_apt) {
            liquidity_pairs::swap_fa_to_apt(name, symbol, transfer_ref, account, fa_metadata_obj, amount_in);
        } else {
            liquidity_pairs::swap_apt_to_fa(name, symbol, transfer_ref, account, fa_metadata_obj, amount_in);
        };
    }

    //---------------------------Internal---------------------------z
    fun create_fa(
        name: String,
        symbol: String,
        max_supply: u128,
        decimals: u8,
        icon_uri: String,
        project_uri: String
    ): (address, FungibleAsset) acquires LaunchPad {
        // Only unique entries of the FA key (name and symbol) can be launched.
        let does_fa_exist = object::object_exists<FAController>(get_fa_obj_address(name, symbol));
        assert!(!does_fa_exist, EFA_EXISTS_ALREADY);
        let launchpad = borrow_global_mut<LaunchPad>(@bonding_curve_launchpad);
        // Obtain the signer from the "FA Generator" object, to create the FA.
        let fa_generator_signer = object::generate_signer_for_extending(&launchpad.fa_generator_extend_ref);
        // The FA's name and symbol is combined, to create a seed for deterministic object creation.
        // Beneficial for retrieving the FA's object address, after the initial creation,
        // like during `get_fa_obj_address(...)`.
        let fa_key_seed = *string::bytes(&name);
        vector::append(&mut fa_key_seed, b"-");
        vector::append(&mut fa_key_seed, *string::bytes(&symbol));
        let fa_obj_constructor_ref = &object::create_named_object(&fa_generator_signer, fa_key_seed);
        let fa_obj_signer = object::generate_signer(fa_obj_constructor_ref);
        // Create the FA and it's associated capabilities within the `bonding_curve_launchpad` account.
        // Plus, mint the predefined amount of FA to be moved to `liquidity_pairs`.
        let base_unit_max_supply = option::some(max_supply * math128::pow(10, (decimals as u128)));
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
        let transfer_ref = fungible_asset::generate_transfer_ref(fa_obj_constructor_ref);
        let fa_minted = fungible_asset::mint(&mint_ref, (max_supply as u64));
        // Define the dispatchable FA's withdraw as a conditionally global freezing effect.
        dispatchable_fungible_asset::register_dispatch_functions(
            fa_obj_constructor_ref,
            option::some(launchpad.permissioned_withdraw),
            option::none(),
            option::none()
        );
        // Store `transfer_ref` for later usage, within the FA's object.
        // `tranfer_ref` will be required to allow the smart contract's modules to ignore the dispatchable
        // withdraw's frozen status for each swap of an FA on `liquidity_pair`.
        move_to(
            &fa_obj_signer,
            FAController { transfer_ref }
        );
        event::emit(
            FungibleAssetCreated {
                name,
                symbol,
                max_supply,
                decimals,
                icon_uri,
                project_uri
            }
        );
        (get_fa_obj_address(name, symbol), fa_minted)
    }

    //---------------------------Tests---------------------------
    #[test_only]
    public fun initialize_for_test(deployer: &signer) {
        let permissioned_withdraw = function_info::new_function_info(
            deployer,
            string::utf8(b"bonding_curve_launchpad"),
            string::utf8(b"withdraw")
        );
        let fa_generator_extend_ref = object::generate_extend_ref(
            &object::create_named_object(deployer, b"FA Generator")
        );
        move_to(
            deployer,
            LaunchPad {
                permissioned_withdraw,
                fa_generator_extend_ref
            }
        );
    }
}
