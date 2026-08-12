/// Example of a managed stablecoin with mint, burn, freeze and pause functionalities.
module stablecoin::usdk {
    use aptos_framework::dispatchable_fungible_asset;
    use aptos_framework::event;
    use aptos_framework::function_info;
    use aptos_framework::fungible_asset::{Self, MintRef, TransferRef, BurnRef, Metadata, FungibleAsset, FungibleStore};
    use aptos_framework::object::{Self, Object, ExtendRef, ObjectCore};
    use aptos_framework::primary_fungible_store;
    use std::option;
    use std::signer;
    use std::string::{Self, utf8};

    /// Caller is not authorized to make this call
    const EUNAUTHORIZED: u64 = 1;
    /// No operations are allowed when contract is paused
    const EPAUSED: u64 = 2;
    /// The account is already a minter
    const EALREADY_MINTER: u64 = 3;
    /// The account is denylisted
    const EDENYLISTED: u64 = 5;
    /// Stablecoin is already initialized
    const EALREADY_INITIALIZED: u64 = 6;
    /// The account has no primary store for the stablecoin
    const ESTORE_NOT_FOUND: u64 = 7;

    const ASSET_SYMBOL: vector<u8> = b"USDK";

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct Roles has key {
        master_minter: address,
        minters: vector<address>,
        pauser: address,
        denylister: address,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct Management has key {
        /// Unused today. Retained so a future upgrade can obtain the metadata object's signer
        /// (e.g. to add resources to the object) without needing to recreate the asset.
        extend_ref: ExtendRef,
        mint_ref: MintRef,
        burn_ref: BurnRef,
        transfer_ref: TransferRef,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct State has key {
        paused: bool,
    }

    struct Approval has drop {
        owner: address,
        to: address,
        nonce: u64,
        chain_id: u8,
        spender: address,
        amount: u64,
    }

    #[event]
    struct Mint has drop, store {
        minter: address,
        to: address,
        amount: u64,
    }

    #[event]
    struct Burn has drop, store {
        minter: address,
        from: address,
        store: Object<FungibleStore>,
        amount: u64,
    }

    #[event]
    struct Pause has drop, store {
        pauser: address,
        is_paused: bool,
    }

    #[event]
    struct Denylist has drop, store {
        denylister: address,
        account: address,
        is_denylisted: bool,
    }

    #[view]
    public fun usdk_address(): address {
        object::create_object_address(&@stablecoin, ASSET_SYMBOL)
    }

    #[view]
    public fun metadata(): Object<Metadata> {
        object::address_to_object(usdk_address())
    }

    #[view]
    /// Return whether the stablecoin is currently paused.
    public fun is_paused(): bool {
        State[usdk_address()].paused
    }

    #[view]
    /// Return whether the account may mint. The master minter is implicitly a minter.
    public fun is_minter(account: address): bool {
        let roles = &Roles[usdk_address()];
        account == roles.master_minter || roles.minters.contains(&account)
    }

    #[view]
    /// Return the accounts explicitly granted the minter role. Excludes the master minter.
    public fun minters(): vector<address> {
        Roles[usdk_address()].minters
    }

    #[view]
    /// Return whether the account is denylisted. Accounts without a primary store are never denylisted.
    public fun is_denylisted(account: address): bool {
        primary_fungible_store::is_frozen(account, metadata())
    }

    #[view]
    public fun master_minter(): address {
        Roles[usdk_address()].master_minter
    }

    #[view]
    public fun pauser(): address {
        Roles[usdk_address()].pauser
    }

    #[view]
    public fun denylister(): address {
        Roles[usdk_address()].denylister
    }

    /// Called as part of deployment to initialize the stablecoin.
    /// Note: The signer has to be the account where the module is published.
    /// Create a stablecoin token (a new Fungible Asset)
    /// Ensure any stores for the stablecoin are untransferable.
    /// Store Roles, Management and State resources in the Metadata object.
    /// Override deposit and withdraw functions of the newly created asset/token to add custom denylist logic.
    entry fun initialize(stablecoin_signer: &signer) {
        // Check that it hasn't been called already
        assert!(!object::object_exists<ObjectCore>(usdk_address()), EALREADY_INITIALIZED);
        authorize(signer::address_of(stablecoin_signer), @stablecoin);

        // Create the stablecoin with primary store support.
        let constructor_ref = &object::create_named_object(stablecoin_signer, ASSET_SYMBOL);
        primary_fungible_store::create_primary_store_enabled_fungible_asset(
            constructor_ref,
            option::none(),
            utf8(ASSET_SYMBOL), /* name */
            utf8(ASSET_SYMBOL), /* symbol */
            6, /* decimals, matching the convention used by USDC/USDT */
            utf8(b"http://example.com/favicon.ico"), /* icon */
            utf8(b"http://example.com"), /* project */
        );

        // Set ALL stores for the fungible asset to untransferable.
        // This prevents secondary stores from being transferred to other accounts.
        fungible_asset::set_untransferable(constructor_ref);

        // All resources created will be kept in the asset metadata object.
        let metadata_object_signer = &constructor_ref.generate_signer();
        move_to(metadata_object_signer, Roles {
            master_minter: @master_minter,
            minters: vector[],
            pauser: @pauser,
            denylister: @denylister,
        });

        // Create mint/burn/transfer refs to allow creator to manage the stablecoin.
        move_to(metadata_object_signer, Management {
            extend_ref: constructor_ref.generate_extend_ref(),
            mint_ref: fungible_asset::generate_mint_ref(constructor_ref),
            burn_ref: fungible_asset::generate_burn_ref(constructor_ref),
            transfer_ref: fungible_asset::generate_transfer_ref(constructor_ref),
        });

        // Start the state not paused
        move_to(metadata_object_signer, State {
            paused: false,
        });

        // Override the deposit and withdraw functions which mean overriding transfer.
        // This ensures all transfer will call withdraw and deposit functions in this module and perform the necessary
        // checks.
        let deposit = function_info::new_function_info(
            stablecoin_signer,
            string::utf8(b"usdk"),
            string::utf8(b"deposit"),
        );
        let withdraw = function_info::new_function_info(
            stablecoin_signer,
            string::utf8(b"usdk"),
            string::utf8(b"withdraw"),
        );
        dispatchable_fungible_asset::register_dispatch_functions(
            constructor_ref,
            option::some(withdraw),
            option::some(deposit),
            option::none(),
        );
    }

    /// Allow a spender to transfer tokens from the owner's account given their signed approval.
    /// Caller needs to provide the from account's scheme and public key which can be gotten via the Aptos SDK.
    /// TODO: This has to be reworked before being re-enabled
    /*public fun transfer_from(
        spender: &signer,
        proof: vector<u8>,
        from: address,
        from_account_scheme: u8,
        from_public_key: vector<u8>,
        to: address,
        amount: u64,
    ) {
        assert_not_paused();
        assert_not_denylisted(from);
        assert_not_denylisted(to);

        let expected_message = Approval {
            owner: from,
            to,
            nonce: account::get_sequence_number(from),
            chain_id: chain_id::get(),
            spender: signer::address_of(spender),
            amount,
        };
        account::verify_signed_message(from, from_account_scheme, from_public_key, proof, expected_message);

        let transfer_ref = &Management[usdk_address()].transfer_ref;
        // Only use with_ref API for primary_fungible_store (PFS) transfers in this module.
        primary_fungible_store::transfer_with_ref(transfer_ref, from, to, amount);
    }*/

    /// Deposit function override to ensure that the account is not denylisted and the stablecoin is not paused.
    public fun deposit<T: key>(
        store: Object<T>,
        fa: FungibleAsset,
        transfer_ref: &TransferRef,
    ) {
        assert_not_paused();
        assert_not_denylisted(store.owner());
        assert_not_denylisted(store.root_owner());
        transfer_ref.deposit_with_ref(store, fa);
    }

    /// Withdraw function override to ensure that the account is not denylisted and the stablecoin is not paused.
    public fun withdraw<T: key>(
        store: Object<T>,
        amount: u64,
        transfer_ref: &TransferRef,
    ): FungibleAsset {
        assert_not_paused();
        assert_not_denylisted(store.owner());
        assert_not_denylisted(store.root_owner());
        transfer_ref.withdraw_with_ref(store, amount)
    }

    /// Mint new tokens to the specified account. This checks that the caller is a minter, the stablecoin is not paused,
    /// and the account is not denylisted.
    public entry fun mint(minter: &signer, to: address, amount: u64) {
        assert_not_paused();
        let minter_address = assert_is_minter(minter);
        assert_not_denylisted(to);

        // Shortcut if you mint 0 tokens, in theory could just assert amount is > 0
        if (amount == 0) { return };

        let management = &Management[usdk_address()];
        let tokens = management.mint_ref.mint(amount);
        // Ensure not to call pfs::deposit or dfa::deposit directly in the module.
        deposit(primary_fungible_store::ensure_primary_store_exists(to, metadata()), tokens, &management.transfer_ref);

        event::emit(Mint {
            minter: minter_address,
            to,
            amount,
        });
    }

    /// Burn tokens from the specified account. This checks that the caller is a minter and the stablecoin is not paused.
    /// Aborts if the account has no primary store rather than creating an empty one as a side effect.
    public entry fun burn(minter: &signer, from: address, amount: u64) {
        let metadata = metadata();
        assert!(primary_fungible_store::primary_store_exists(from, metadata), ESTORE_NOT_FOUND);
        burn_from(minter, primary_fungible_store::primary_store(from, metadata), amount);
    }

    /// Burn tokens from the specified account's store. This checks that the caller is a minter and the stablecoin is
    /// not paused.
    public entry fun burn_from(
        minter: &signer,
        store: Object<FungibleStore>,
        amount: u64,
    ) {
        assert_not_paused();
        let minter_address = assert_is_minter(minter);
        if (amount == 0) { return };

        let management = &Management[usdk_address()];
        let tokens = management.transfer_ref.withdraw_with_ref(store, amount);
        management.burn_ref.burn(tokens);

        event::emit(Burn {
            minter: minter_address,
            from: store.owner(),
            store,
            amount,
        });
    }

    /// Pause or unpause the stablecoin. This checks that the caller is the pauser.
    public entry fun set_pause(pauser: &signer, paused: bool) {
        let usdk_address = usdk_address();
        let pauser_address = signer::address_of(pauser);
        let roles = &Roles[usdk_address];
        authorize(pauser_address, roles.pauser);
        let state = &mut State[usdk_address];

        // Idempotent return if the paused state is already the same as the requested state.
        if (state.paused == paused) { return };
        state.paused = paused;

        event::emit(Pause {
            pauser: pauser_address,
            is_paused: paused,
        });
    }

    /// Add an account to the denylist. This checks that the caller is the denylister.
    public entry fun denylist(denylister: &signer, account: address) {
        set_denylist(denylister, account, true);
    }

    /// Remove an account from the denylist. This checks that the caller is the denylister.
    public entry fun undenylist(denylister: &signer, account: address) {
        set_denylist(denylister, account, false);
    }

    inline fun set_denylist(denylister: &signer, account: address, denied: bool) {
        let usdk_address = usdk_address();
        let denylister_address = signer::address_of(denylister);
        let roles = &Roles[usdk_address];
        authorize(denylister_address, roles.denylister);

        let transfer_ref = &Management[usdk_address].transfer_ref;
        primary_fungible_store::set_frozen_flag(transfer_ref, account, denied);

        event::emit(Denylist {
            denylister: denylister_address,
            account,
            is_denylisted: denied
        });
    }

    /// Add a new minter. This checks that the caller is the master minter and the account is not already a minter.
    /// Deliberately callable while paused: revoking and granting roles is part of incident response.
    public entry fun add_minter(admin: &signer, minter: address) {
        let admin_address = signer::address_of(admin);
        let roles = &mut Roles[usdk_address()];
        authorize(admin_address, roles.master_minter);
        assert!(!roles.minters.contains(&minter), EALREADY_MINTER);
        roles.minters.push_back(minter);
    }

    /// Revoke the minter role. This checks that the caller is the master minter. No-op if the account is not a minter.
    /// Deliberately callable while paused: revoking a compromised minter is part of incident response.
    public entry fun remove_minter(admin: &signer, minter: address) {
        let admin_address = signer::address_of(admin);
        let roles = &mut Roles[usdk_address()];
        authorize(admin_address, roles.master_minter);
        let (found, index) = roles.minters.index_of(&minter);
        if (found) {
            // Order doesn't matter, so let's do the performant remove
            roles.minters.swap_remove(index);
        };
    }

    inline fun assert_is_minter(minter: &signer): address {
        let roles = &Roles[usdk_address()];
        let minter_addr = signer::address_of(minter);
        assert!(minter_addr == roles.master_minter || roles.minters.contains(&minter_addr), EUNAUTHORIZED);
        minter_addr
    }

    inline fun assert_not_paused() {
        let state = &State[usdk_address()];
        assert!(!state.paused, EPAUSED);
    }

    // Check that the account is not denylisted by checking the frozen flag on the primary store
    inline fun assert_not_denylisted(account: address) {
        let metadata = metadata();
        // CANNOT call into pfs::store_exists in our withdraw/deposit hooks as it creates possibility of a circular dependency.
        // Instead, we will call the inlined version of the function.
        if (primary_fungible_store::primary_store_exists_inlined(account, metadata)) {
            assert!(
                !fungible_asset::is_frozen(primary_fungible_store::primary_store_inlined(account, metadata)),
                EDENYLISTED
            );
        }
    }

    inline fun authorize(caller: address, expected: address) {
        assert!(caller == expected, EUNAUTHORIZED);
    }

    #[test_only]
    public fun init_for_test(usdk_signer: &signer) {
        initialize(usdk_signer);
    }
}
