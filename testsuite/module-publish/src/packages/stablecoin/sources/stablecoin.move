// MIT License
// Copyright (c) 2024 Aptos
// Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the Software without restriction, including without limitation the
// rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to
// permit persons to whom the Software is furnished to do so, subject to the following conditions:
// The above copyright notice and this permission notice shall be included in all copies or substantial portions of the
// Software.
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE
// WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. NOTWITHSTANDING ANYTHING TO THE
// CONTRARY, IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY,
// WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE
// USE OR OTHER DEALINGS IN THE SOFTWARE.

module stablecoin::stablecoin {
    use std::signer;
    use std::vector;
    use std::option::{Self, Option};
    use std::string::{Self, utf8};

    use aptos_std::smart_table::{Self, SmartTable};

    use aptos_framework::event;
    use aptos_framework::primary_fungible_store;
    use aptos_framework::dispatchable_fungible_asset;
    use aptos_framework::function_info;
    use aptos_framework::object::{Self, ExtendRef, Object};
    use aptos_framework::fungible_asset::{
        Self,
        BurnRef,
        FungibleAsset,
        Metadata,
        MintRef,
        MutateMetadataRef,
        TransferRef
    };

    use stablecoin::pausable;
    use stablecoin::ownable;

    // ===== Errors and Constants =====

    /// Address is not the master minter.
    const ENOT_MASTER_MINTER: u64 = 1;
    /// Address is not a controller.
    const ENOT_CONTROLLER: u64 = 2;
    /// Address is not a minter.
    const ENOT_MINTER: u64 = 3;
    /// Amount must be greater than zero.
    const EZERO_AMOUNT: u64 = 4;
    /// Address is denylisted.
    const EDENYLISTED: u64 = 5;
    /// Insufficient mint allowance.
    const EINSUFFICIENT_ALLOWANCE: u64 = 6;
    /// Incorrect length of parameters.
    const EVECTOR_LENGTH_MISMATCH: u64 = 7;
    /// Address is not the denylister.
    const ENOT_DENYLISTER: u64 = 8;

    const STABLECOIN_NAME: vector<u8> = b"Stablecoin";
    const STABLECOIN_SYMBOL: vector<u8> = b"Stablecoin";
    const STABLECOIN_DECIMALS: u8 = 6;
    const PROJECT_URI: vector<u8> = b"https://tether.to"; // TODO: Update PROJECT URI
const ICON_URI: vector<u8> = b"https://tether.to/images/logoCircle.png"; // TODO: Update ICON URI

    // ===== Resources =====

    struct Management has key {
        /// The capapbility to update the metadata object's storage.
        extend_ref: ExtendRef,
        /// The capapbility to mint units of the coin.
        mint_ref: MintRef,
        /// The capapbility to burn units of the coin.
        burn_ref: BurnRef,
        /// The capbility to transfer and freeze units of the coin.
        transfer_ref: TransferRef,
        /// The capbility to update the metadata object's metadata resource.
        mutate_metadata_ref: MutateMetadataRef
    }

    struct Roles has key {
        /// The address of the master minter.
        denylister: address,
        /// The address of the master minter.
        master_minter: address,
        /// The address that of the metadata updater.
        metadata_updater: address
    }

    struct State has key {
        /// Mapping containing minters and their allowances.
        mint_allowances: SmartTable<address, u64>,
        /// Mapping containing controllers and the minter addresses they control.
        controllers: SmartTable<address, address>
    }

    // ===== Events =====

    #[event]
    struct Deposit has drop, store {
        metadata_address: address,
        store_owner: address,
        store: address,
        amount: u64
    }

    #[event]
    struct Withdraw has drop, store {
        metadata_address: address,
        store_owner: address,
        store: address,
        amount: u64
    }

    #[event]
    struct ControllerConfigured has drop, store {
        controller: address,
        minter: address
    }

    #[event]
    struct ControllerRemoved has drop, store {
        controller: address
    }

    #[event]
    struct MinterConfigured has drop, store {
        controller: address,
        minter: address,
        allowance: u64
    }

    #[event]
    struct MinterAllowanceIncremented has drop, store {
        controller: address,
        minter: address,
        allowance_increment: u64,
        new_allowance: u64
    }

    #[event]
    struct MinterRemoved has drop, store {
        controller: address,
        minter: address
    }

    #[event]
    struct MasterMinterChanged has drop, store {
        old_master_minter: address,
        new_master_minter: address
    }

    #[event]
    struct Mint has drop, store {
        minter: address,
        to: address,
        amount: u64
    }

    #[event]
    struct Burn has drop, store {
        minter: address,
        from: address,
        amount: u64
    }

    #[event]
    struct Denylisted has drop, store {
        address: address
    }

    #[event]
    struct Undenylisted has drop, store {
        address: address
    }

    #[event]
    struct DenylisterChanged has drop, store {
        old_denylister: address,
        new_denylister: address
    }

    // ===== View Functions =====

    #[view]
    public fun balance_of(account: address): u64 {
        primary_fungible_store::balance(account, stablecoin_object())
    }

    #[view]
    public fun stablecoin_address(): address {
        object::create_object_address(&@stablecoin, STABLECOIN_SYMBOL)
    }

    #[view]
    public fun stablecoin_object(): Object<Metadata> {
        object::address_to_object(stablecoin_address())
    }

    #[view]
    public fun master_minter(): address acquires Roles {
        borrow_global<Roles>(stablecoin_address()).master_minter
    }

    #[view]
    public fun get_minter(controller: address): Option<address> acquires State {
        if (!is_controller_inlined(controller)) {
            return option::none()
        };
        option::some(get_minter_inlined(controller))
    }

    #[view]
    public fun is_minter(minter: address): bool acquires State {
        let supply_manager = borrow_global<State>(stablecoin_address());
        smart_table::contains(&supply_manager.mint_allowances, minter)
    }

    #[view]
    public fun mint_allowance(minter: address): u64 acquires State {
        if (!is_minter_inlined(minter)) {
            return 0
        };
        mint_allowance_inlined(minter)
    }

    // ===== Initialization =====

    /// Creates Stable coin during package deployment.
    fun init_module(code_object: &signer) {
        // Create the fungible asset with primary store support.
        let constructor_ref = &object::create_named_object(code_object, STABLECOIN_SYMBOL);
        primary_fungible_store::create_primary_store_enabled_fungible_asset(
            constructor_ref,
            option::none(),
            utf8(STABLECOIN_NAME),
            utf8(STABLECOIN_SYMBOL),
            STABLECOIN_DECIMALS,
            string::utf8(ICON_URI),
            string::utf8(PROJECT_URI)
        );

        // Set all stores derived from the asset as untransferable.
        fungible_asset::set_untransferable(constructor_ref);

        // All resources created will be moved to the asset metadata object.
        let metadata_object_signer = &object::generate_signer(constructor_ref);

        // Create pausable and ownable state.
        ownable::new(metadata_object_signer, @stablecoin);
        pausable::new(metadata_object_signer, @master_minter);

        // Generate and store the refs for asset management.
        move_to(
            metadata_object_signer,
            Management {
                extend_ref: object::generate_extend_ref(constructor_ref),
                mint_ref: fungible_asset::generate_mint_ref(constructor_ref),
                burn_ref: fungible_asset::generate_burn_ref(constructor_ref),
                transfer_ref: fungible_asset::generate_transfer_ref(constructor_ref),
                mutate_metadata_ref: fungible_asset::generate_mutate_metadata_ref(
                    constructor_ref
                )
            }
        );

        // Generate and store the roles for the asset.
        move_to(
            metadata_object_signer,
            Roles {
                denylister: @denylister,
                master_minter: @master_minter,
                metadata_updater: @metadata_updater
            }
        );

        // Generate and store the state for the asset.
        move_to(
            metadata_object_signer,
            State {
                mint_allowances: smart_table::new(),
                controllers: smart_table::new(),
            }
        );

        // Create overrides for deposit and withdraw functions - which means overriding transfer.
        let deposit_function_info =
            function_info::new_function_info(
                code_object,
                utf8(b"stablecoin"),
                utf8(b"override_deposit")
            );
        let withdraw_function_info =
            function_info::new_function_info(
                code_object,
                utf8(b"stablecoin"),
                utf8(b"override_withdraw")
            );

        // Register the custom deposit and withdraw functions.
        dispatchable_fungible_asset::register_dispatch_functions(
            constructor_ref,
            option::some(withdraw_function_info),
            option::some(deposit_function_info),
            option::none() /* omit override for derived_balance */
        );
    }

    // ===== Overrides =====

    /// Override deposit to verify pause and denylist status.
    public fun override_deposit<T: key>(
        store: Object<T>, fa: FungibleAsset, transfer_ref: &TransferRef
    ) {
        let metadata_address = stablecoin_address();
        let store_owner = object::owner(store);
        let store_address = object::object_address(&store);
        let amount = fungible_asset::amount(&fa);

        pausable::assert_not_paused(metadata_address);
        assert_not_denylisted(store_owner);
        fungible_asset::deposit_with_ref(transfer_ref, store, fa);

        event::emit(
            Deposit { metadata_address, store_owner, store: store_address, amount }
        )
    }

    /// Override withdraw to verify pause and denylist status.
    public fun override_withdraw<T: key>(
        store: Object<T>, amount: u64, transfer_ref: &TransferRef
    ): FungibleAsset {
        let metadata_address = stablecoin_address();
        let store_owner = object::owner(store);
        let store_address = object::object_address(&store);

        pausable::assert_not_paused(metadata_address);
        assert_not_denylisted(store_owner);
        let asset = fungible_asset::withdraw_with_ref(transfer_ref, store, amount);

        event::emit(
            Withdraw { metadata_address, store_owner, store: store_address, amount }
        );

        asset
    }

    // ===== Controller and Minter Configuration =====

    /// Update master minter role.
    public entry fun update_master_minter(
        caller: &signer, new_master_minter: address
    ) acquires Roles {
        ownable::assert_is_owner(caller, stablecoin_address());

        let roles = borrow_global_mut<Roles>(stablecoin_address());
        let old_master_minter = roles.master_minter;
        roles.master_minter = new_master_minter;

        event::emit(MasterMinterChanged { old_master_minter, new_master_minter });
    }

    /// Configures the controller for a minter.
    /// Controller to Minter is a one-to-one relationship.
    /// Minter to Controller can be a one-to-many relationship.
    public entry fun configure_controller(
        controller: &signer
    ) acquires State {
        // let caller_address = signer::address_of(caller);
        // assert_master_minter(caller_address);

        let controller_address = signer::address_of(controller);
        let supply_manager = borrow_global_mut<State>(stablecoin_address());
        smart_table::upsert(&mut supply_manager.controllers, controller_address, controller_address);

        event::emit(ControllerConfigured { controller: controller_address, minter: controller_address });
    }

    /// Removes a controller.
    public entry fun remove_controller(caller: &signer, controller: address) acquires State, Roles {
        let caller_address = signer::address_of(caller);
        assert_master_minter(caller_address);
        assert_is_controller(controller);

        let supply_manager = borrow_global_mut<State>(stablecoin_address());
        smart_table::remove(&mut supply_manager.controllers, controller);

        event::emit(ControllerRemoved { controller });
    }

    /// Configures a minter by setting an allowance,
    /// allowing the address to mint and burn the coin.
    public entry fun configure_minter(caller: &signer, allowance: u64) acquires State {
        pausable::assert_not_paused(stablecoin_address());

        // let controller = signer::address_of(caller);
        // assert_is_controller(controller);

        // let minter = get_minter_inlined(controller);
        let controller = signer::address_of(caller);
        let minter = signer::address_of(caller);
        set_mint_allowance(minter, allowance);

        event::emit(MinterConfigured { controller, minter, allowance });
    }

    /// Increment the allowance for a minter.
    public entry fun increment_minter_allowance(
        caller: &signer, allowance_increment: u64
    ) acquires State {
        pausable::assert_not_paused(stablecoin_address());
        assert!(allowance_increment > 0, EZERO_AMOUNT);

        let controller = signer::address_of(caller);
        assert_is_controller(controller);

        let minter = get_minter_inlined(controller);
        assert_is_minter(minter);

        let new_allowance = mint_allowance_inlined(minter) + allowance_increment;
        set_mint_allowance(minter, new_allowance);

        event::emit(
            MinterAllowanceIncremented {
                controller,
                minter,
                allowance_increment,
                new_allowance
            }
        );
    }

    /// Removes a minter.
    public entry fun remove_minter(caller: &signer) acquires State {
        let controller = signer::address_of(caller);
        assert_is_controller(controller);

        let minter = get_minter_inlined(controller);
        assert_is_minter(minter);

        let supply_manager = borrow_global_mut<State>(stablecoin_address());
        smart_table::remove(&mut supply_manager.mint_allowances, minter);

        event::emit(MinterRemoved { controller, minter });
    }

    /// Sets the mint allowance for a minter.
    fun set_mint_allowance(minter: address, allowance: u64) acquires State {
        let supply_manager = borrow_global_mut<State>(stablecoin_address());
        smart_table::upsert(&mut supply_manager.mint_allowances, minter, allowance);
    }

    // ===== Minting and Burning =====

    /// Mints the stable coin and returns the minted FA.
    /// The amount minted is limited to the minter's allowance.
    /// Increases the total supply and decreases the allowance.
    public entry fun mint(caller: &signer, to: address, amount: u64) acquires Management, State {
        assert!(amount > 0, EZERO_AMOUNT);
        pausable::assert_not_paused(stablecoin_address());

        let minter = signer::address_of(caller);
        assert_is_minter(minter);
        assert_not_denylisted(to);

        let mint_allowance = mint_allowance_inlined(minter);
        assert!(mint_allowance >= amount, EINSUFFICIENT_ALLOWANCE);

        let mint_ref = &borrow_global<Management>(stablecoin_address()).mint_ref;
        let to_store = primary_fungible_store::ensure_primary_store_exists(to, stablecoin_object());
        fungible_asset::mint_to(mint_ref, to_store, amount);

        set_mint_allowance(minter, mint_allowance - amount);

        event::emit(Mint { minter, to, amount });
    }

    /// Mints the stable coin to multiple addresses.
    public entry fun batch_mint(
        caller: &signer, addresses: vector<address>, amounts: vector<u64>
    ) acquires Management, State {
        pausable::assert_not_paused(stablecoin_address());

        let minter = signer::address_of(caller);
        assert_is_minter(minter);

        assert!(
            vector::length(&addresses) == vector::length(&amounts), EVECTOR_LENGTH_MISMATCH
        );

        let mint_ref = &borrow_global<Management>(stablecoin_address()).mint_ref;
        for (i in 0..vector::length(&addresses)) {
            let to = *vector::borrow(&addresses, i);
            assert_not_denylisted(to);

            let amount = *vector::borrow(&amounts, i);
            assert!(amount > 0, EZERO_AMOUNT);

            let mint_allowance = mint_allowance_inlined(minter);
            assert!(mint_allowance >= amount, EINSUFFICIENT_ALLOWANCE);

            let to_store =
                primary_fungible_store::ensure_primary_store_exists(to, stablecoin_object());
            fungible_asset::mint_to(mint_ref, to_store, amount);

            set_mint_allowance(minter, mint_allowance - amount);

            event::emit(Mint { minter, to, amount });
        }
    }

    public entry fun burn(caller: &signer, from: address, amount: u64) acquires Management, State {
        assert!(amount > 0, EZERO_AMOUNT);
        pausable::assert_not_paused(stablecoin_address());

        let minter = signer::address_of(caller);
        assert_is_minter(minter);
        assert_not_denylisted(minter);

        let burn_ref = &borrow_global<Management>(stablecoin_address()).burn_ref;
        let from_store =
            primary_fungible_store::ensure_primary_store_exists(from, stablecoin_object());
        fungible_asset::burn_from(burn_ref, from_store, amount);

        event::emit(Burn { minter, from, amount });
    }

    // ===== Denylisting =====

    public entry fun denylist(caller: &signer, account: address) acquires Management, Roles {
        let caller_address = signer::address_of(caller);
        assert_is_denylister(caller_address);

        let freeze_ref = &borrow_global<Management>(stablecoin_address()).transfer_ref;
        primary_fungible_store::set_frozen_flag(freeze_ref, account, true);

        event::emit(Denylisted { address: account })
    }

    public entry fun undenylist(caller: &signer, account: address) acquires Management, Roles {
        let caller_address = signer::address_of(caller);
        assert_is_denylister(caller_address);

        let freeze_ref = &borrow_global<Management>(stablecoin_address()).transfer_ref;
        primary_fungible_store::set_frozen_flag(freeze_ref, account, false);

        event::emit(Undenylisted { address: account })
    }

    public entry fun update_denylister(caller: &signer, new_denylister: address) acquires Roles {
        ownable::assert_is_owner(caller, stablecoin_address());

        let roles = borrow_global_mut<Roles>(stablecoin_address());
        let old_denylister = roles.denylister;
        roles.denylister = new_denylister;

        event::emit(DenylisterChanged { old_denylister, new_denylister });
    }

    // ===== Inline Functions =====

    inline fun is_controller_inlined(controller: address): bool acquires State {
        let supply_manager = borrow_global<State>(stablecoin_address());
        smart_table::contains(&supply_manager.controllers, controller)
    }

    inline fun is_minter_inlined(minter: address): bool acquires State {
        let supply_manager = borrow_global<State>(stablecoin_address());
        smart_table::contains(&supply_manager.mint_allowances, minter)
    }

    inline fun get_minter_inlined(controller: address): address acquires State {
        let supply_manager = borrow_global<State>(stablecoin_address());
        *smart_table::borrow(&supply_manager.controllers, controller)
    }

    inline fun mint_allowance_inlined(minter: address): u64 acquires State {
        let supply_manager = borrow_global<State>(stablecoin_address());
        *smart_table::borrow(&supply_manager.mint_allowances, minter)
    }

    inline fun assert_master_minter(caller: address) acquires Roles {
        assert!(caller == master_minter(), ENOT_MASTER_MINTER);
    }

    inline fun assert_is_controller(caller: address) acquires State {
        let supply_manager = borrow_global<State>(stablecoin_address());
        assert!(
            smart_table::contains(&supply_manager.controllers, caller), ENOT_CONTROLLER
        );
    }

    inline fun assert_is_minter(caller: address) acquires State {
        let supply_manager = borrow_global<State>(stablecoin_address());
        assert!(
            smart_table::contains(&supply_manager.mint_allowances, caller), ENOT_MINTER
        );
    }

    inline fun assert_is_denylister(caller: address) acquires Roles {
        let denylister = &borrow_global<Roles>(stablecoin_address()).denylister;
        assert!(&caller == denylister, ENOT_DENYLISTER);
    }

    inline fun assert_not_denylisted(account: address) acquires State {
        let stablecoin_object = stablecoin_object();
        // If the account's primary store is frozen, then the account is denylisted.
        if (primary_fungible_store::primary_store_exists_inlined(account, stablecoin_object)) {
            let account_primary_store =
                primary_fungible_store::primary_store_inlined(account, stablecoin_object);
            let is_denylisted = fungible_asset::is_frozen(account_primary_store);
            assert!(!is_denylisted, EDENYLISTED);
        }
    }

    #[test_only]
    public fun init_for_test(caller: &signer) {
        init_module(caller);
    }
}