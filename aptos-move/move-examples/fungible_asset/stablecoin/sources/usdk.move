/// Example of a managed stablecoin with mint, burn, freeze and pause functionalities.
module stablecoin::usdk {
    use aptos_framework::account;
    use aptos_framework::dispatchable_fungible_asset;
    use aptos_framework::event;
    use aptos_framework::function_info;
    use aptos_framework::fungible_asset::{Self, MintRef, TransferRef, BurnRef, Metadata, FungibleAsset};
    use aptos_framework::object::{Self, Object, ExtendRef};
    use aptos_framework::primary_fungible_store;
    use aptos_std::smart_table::{Self, SmartTable};
    use std::option;
    use std::signer;
    use std::string::utf8;
    use std::string;
    use std::vector;

    /// Caller is not authorized to make this call
    const EUNAUTHORIZED: u64 = 1;
    /// No operations are allowed when contract is paused
    const EPAUSED: u64 = 2;
    /// The account is already a minter
    const EALREADY_MINTER: u64 = 3;
    /// The account is not a minter
    const ENOT_MINTER: u64 = 4;
    /// The account is blacklisted
    const EBLACKLISTED: u64 = 5;

    const ASSET_SYMBOL: vector<u8> = b"USDK";

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct Roles has key {
        master_minter: address,
        minters: vector<address>,
        pauser: address,
        blacklister: address,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct Management has key {
        extend_ref: ExtendRef,
        mint_ref: MintRef,
        burn_ref: BurnRef,
        transfer_ref: TransferRef,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct State has key {
        paused: bool,
        blacklist: SmartTable<address, bool>,
    }

    struct Approval has drop {
        owner: address,
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
        amount: u64,
    }

    #[event]
    struct Pause has drop, store {
        pauser: address,
        paused: bool,
    }

    #[event]
    struct Blacklist has drop, store {
        blacklister: address,
        account: address,
    }

    #[view]
    public fun usdk_address(): address {
        object::create_object_address(&@stablecoin, ASSET_SYMBOL)
    }

    #[view]
    public fun metadata(): Object<Metadata> {
        object::address_to_object(usdk_address())
    }

    fun init_module(usdk_signer: &signer) {
        let constructor_ref = &object::create_named_object(usdk_signer, ASSET_SYMBOL);
        primary_fungible_store::create_primary_store_enabled_fungible_asset(
            constructor_ref,
            option::none(),
            utf8(ASSET_SYMBOL), /* name */
            utf8(ASSET_SYMBOL), /* symbol */
            8, /* decimals */
            utf8(b"http://example.com/favicon.ico"), /* icon */
            utf8(b"http://example.com"), /* project */
        );

        // All resources created will be kept in the asset metadata object.
        let metadata_object_signer = &object::generate_signer(constructor_ref);
        move_to(metadata_object_signer, Roles {
            master_minter: @master_minter,
            minters: vector[@minter],
            pauser: @pauser,
            blacklister: @blacklister,
        });

        // Create mint/burn/transfer refs to allow creator to manage the stablecoin.
        move_to(metadata_object_signer, Management {
            extend_ref: object::generate_extend_ref(constructor_ref),
            mint_ref: fungible_asset::generate_mint_ref(constructor_ref),
            burn_ref: fungible_asset::generate_burn_ref(constructor_ref),
            transfer_ref: fungible_asset::generate_transfer_ref(constructor_ref),
        });

        move_to(metadata_object_signer, State {
            paused: false,
            blacklist: smart_table::new(),
        });

        // Overrides
        let deposit = function_info::new_function_info(
            usdk_signer,
            string::utf8(b"usdk"),
            string::utf8(b"deposit"),
        );
        let withdraw = function_info::new_function_info(
            usdk_signer,
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

    public fun transfer_from(
        spender: &signer,
        proof: vector<u8>,
        from: address,
        from_account_scheme: u8,
        from_public_key: vector<u8>,
        to: address,
        amount: u64,
    ) acquires Management, State {
        assert_not_paused();
        assert_not_blacklisted(from);
        assert_not_blacklisted(to);

        let expected_message = Approval {
            owner: from,
            spender: signer::address_of(spender),
            amount,
        };
        account::verify_signed_message(from, from_account_scheme, from_public_key, proof, expected_message);

        let transfer_ref = &borrow_global<Management>(usdk_address()).transfer_ref;
        primary_fungible_store::transfer_with_ref(transfer_ref, from, to, amount);
    }

    public fun deposit<T: key>(
        store: Object<T>,
        fa: FungibleAsset,
        transfer_ref: &TransferRef,
    ) acquires State {
        assert_not_paused();
        assert_not_blacklisted(object::owner(store));
        fungible_asset::deposit_with_ref(transfer_ref, store, fa);
    }

    public fun withdraw<T: key>(
        store: Object<T>,
        amount: u64,
        transfer_ref: &TransferRef,
    ): FungibleAsset acquires State {
        assert_not_paused();
        assert_not_blacklisted(object::owner(store));
        fungible_asset::withdraw_with_ref(transfer_ref, store, amount)
    }

    public entry fun mint(minter: &signer, to: address, amount: u64) acquires Management, Roles, State {
        assert_is_minter(minter);
        assert_not_paused();
        assert_not_blacklisted(to);

        let management = borrow_global<Management>(usdk_address());
        let tokens = fungible_asset::mint(&management.mint_ref, amount);
        deposit(primary_fungible_store::ensure_primary_store_exists(to, metadata()), tokens, &management.transfer_ref);

        event::emit(Mint {
            minter: signer::address_of(minter),
            to,
            amount,
        });
    }

    public entry fun burn(minter: &signer, from: address, amount: u64) acquires Management, Roles, State {
        assert_is_minter(minter);
        assert_not_paused();
        let management = borrow_global<Management>(usdk_address());
        let tokens = fungible_asset::withdraw_with_ref(
            &management.transfer_ref,
            primary_fungible_store::ensure_primary_store_exists(from, metadata()),
            amount,
        );
        fungible_asset::burn(&management.burn_ref, tokens);

        event::emit(Burn {
            minter: signer::address_of(minter),
            from,
            amount,
        });
    }

    public entry fun set_pause(pauser: &signer, paused: bool) acquires Roles, State {
        let roles = borrow_global<Roles>(usdk_address());
        assert!(signer::address_of(pauser) == roles.pauser, EUNAUTHORIZED);
        let state = borrow_global_mut<State>(usdk_address());
        state.paused = paused;

        event::emit(Pause {
            pauser: signer::address_of(pauser),
            paused,
        });
    }

    public entry fun blacklist(blacklister: &signer, account: address) acquires Management, Roles, State {
        assert_not_paused();
        let roles = borrow_global<Roles>(usdk_address());
        assert!(signer::address_of(blacklister) == roles.blacklister, EUNAUTHORIZED);
        let state = borrow_global_mut<State>(usdk_address());
        smart_table::upsert(&mut state.blacklist, account, true);

        let freeze_ref = &borrow_global<Management>(usdk_address()).transfer_ref;
        primary_fungible_store::set_frozen_flag(freeze_ref, account, true);

        event::emit(Blacklist {
            blacklister: signer::address_of(blacklister),
            account,
        });
    }

    public entry fun unblacklist(blacklister: &signer, account: address) acquires Management, Roles, State {
        assert_not_paused();
        let roles = borrow_global<Roles>(usdk_address());
        assert!(signer::address_of(blacklister) == roles.blacklister, EUNAUTHORIZED);
        let state = borrow_global_mut<State>(usdk_address());
        smart_table::remove(&mut state.blacklist, account);

        let freeze_ref = &borrow_global<Management>(usdk_address()).transfer_ref;
        primary_fungible_store::set_frozen_flag(freeze_ref, account, false);

        event::emit(Blacklist {
            blacklister: signer::address_of(blacklister),
            account,
        });
    }

    public entry fun add_minter(admin: &signer, minter: address) acquires Roles {
        let roles = borrow_global_mut<Roles>(usdk_address());
        assert!(signer::address_of(admin) == roles.master_minter, EUNAUTHORIZED);
        assert!(!vector::contains(&roles.minters, &minter), EALREADY_MINTER);
        vector::push_back(&mut roles.minters, minter);
    }

    fun assert_is_minter(minter: &signer) acquires Roles {
        let roles = borrow_global<Roles>(usdk_address());
        let minter = signer::address_of(minter);
        assert!(minter == roles.master_minter || vector::contains(&roles.minters, &minter), EUNAUTHORIZED);
    }

    fun assert_not_paused() acquires State {
        let state = borrow_global<State>(usdk_address());
        assert!(!state.paused, EPAUSED);
    }

    fun assert_not_blacklisted(account: address) acquires State {
        let state = borrow_global<State>(usdk_address());
        assert!(!smart_table::contains(&state.blacklist, account), EBLACKLISTED);
    }

    #[test_only]
    public fun init_for_test(usdk_signer: &signer) {
        init_module(usdk_signer);
    }
}
