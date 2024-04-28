module circle::fiat_token {
    use aptos_framework::fungible_asset::{Self, MintRef, TransferRef, BurnRef, Metadata};
    use aptos_framework::object::{Self, Object};
    use aptos_framework::primary_fungible_store;
    use std::error;
    use std::string::{Self, String};
    use std::option;
    use std::signer;
    use std::vector;
    use aptos_std::smart_table::{Self, SmartTable};

    friend circle::master_minter;

    /// Error codes documentation: https://aptos.dev/reference/error-codes/
    const EALREADY_INITIALIZED: u64 = 1;
    const EBLACKLISTED_ADDRESS: u64 = 2;
    const ECALLER_NOT_MINTER: u64 = 3;
    const EMINTER_ALLOWANCE_EXCEEDED: u64 = 4;
    const EINSUFFICIENT_BALANCE: u64 = 5;
    const ENOT_MASTER_MINTER: u64 = 6;
    const ENOT_OWNER: u64 = 7;

    /// Asset Symbol
    const ASSET_SYMBOL: vector<u8> = b"USDC";

    /// Module Config for Initializer
    struct ModuleConfig has key {
        is_initialized: bool,
    }

    /// Store refs to Manage Fiat Token
    struct FiatTokenConfig has key {
        mint_ref: MintRef,
        burn_ref: BurnRef,
        transfer_ref: TransferRef,
    }

    /// Store access info to Manage Fiat Token
    struct FiatTokenAccessInfo has key {
        minters: SmartTable<address, u64>,
        blacklist: vector<address>,
    }


    /// TODO: store ALL roles under a named object. 

    /// Store under owner address
    struct MasterMinterConfig has key {
        master_minter: address,
    }

    // struct Blacklister has key, store { }
    // struct Roles have key { } 

    /// Initialize metadata object and store it in the `Metadata` resource
    /// This function is called only once when the module is deployed
    /// Only accessible by the REST API
    /// Note: owner needs to be the circle account's signer reference
    entry fun initialize(
        owner: &signer,
        master_minter: address,
        maximum_supply: u128,
        name: String,
        symbol: String,
        decimals: u8,
    ) {
        // check if module is already initialized
        assert!(!exists<FiatTokenConfig>(@circle), error::invalid_state(EALREADY_INITIALIZED));
        // assert!(!borrow_global<ModuleConfig>(@circle).is_initialized, error::invalid_state(EALREADY_INITIALIZED));

        // move ModuleConfig to owner's account and set `is_initialized` to true
        move_to(owner, ModuleConfig { is_initialized: true });

        // if `maximum_supply` is/not 0, then `Option` is created without/with a value
        // using an `Option` type signals that a `maximum_supply` of 0 indicates an unlimited supply
        let supply = if (maximum_supply != 0) {
            option::some(maximum_supply) // limited supply
        } else {
            option::none() // unlimited supply
        };

        // creates named object (address of object is created using `creator address` and user generated `seed`) and returns the ConstructorRef
        // `create_primary_store_enabled_fungible_asset` creates a fungible asset with primary store support by doing the following:
        // ConstructorRef is used to generate an object signer, to create `Metadata` resource and store at the `Metadata object's` address, 
        // and also store `DeriveRefPod` Resource within the object (at the same address)
        let constructor_ref = &object::create_named_object(owner, ASSET_SYMBOL);
        primary_fungible_store::create_primary_store_enabled_fungible_asset(
            constructor_ref,
            supply,
            name,
            symbol,
            decimals,
            string::utf8(b""),
            string::utf8(b"")
        );

        // generate MintRef, BurnRef, and TransferRef using ConstructorRef
        let mint_ref = fungible_asset::generate_mint_ref(constructor_ref);
        let burn_ref = fungible_asset::generate_burn_ref(constructor_ref);
        let transfer_ref = fungible_asset::generate_transfer_ref(constructor_ref);

        // store refs in `FiatTokenConfig` resource, move resource to `Metadata object` address
        let metadata_object_signer = object::generate_signer(constructor_ref);
        move_to(
            &metadata_object_signer,
            FiatTokenConfig { mint_ref, burn_ref, transfer_ref },
        );
        // store access info in `FiatTokenAccessInfo` resource, move resource to `Circle/Owner` address
        move_to(
            owner,
            FiatTokenAccessInfo {
                minters: smart_table::new(),
                blacklist: vector::empty(),
            },
        );
        // store master minter in `MasterMinter` resource, move resource to `Circle/Owner` address
        move_to(
            owner,
            MasterMinterConfig { master_minter },
        )
    }
    
    /// Mint new fiat tokens and deposit them to the `to` address.
    /// Returns true if operation is successful.
    public entry fun mint(minter: &signer, to: address, amount: u64) acquires FiatTokenConfig, FiatTokenAccessInfo  {
        // SECURITY TODO: check if contract is paused 
        // SECURITY TODO: check if transaction authorizer (signer) is a minter OR owner

        // check if `minter` or `to` address is blacklisted
        assert!(!is_blacklisted(signer::address_of(minter)), error::permission_denied(EBLACKLISTED_ADDRESS));
        assert!(!is_blacklisted(to), error::permission_denied(EBLACKLISTED_ADDRESS));
        
        // check if amount is greater than the minter's allowance 
        assert!(get_minter_allowance(signer::address_of(minter)) > amount, error::permission_denied(EMINTER_ALLOWANCE_EXCEEDED));
        
        // retrieve the refs using the `Metadata` object
        let metadata_object = get_metadata_object();
        let fiat_token_config = authorized_borrow_refs(minter, metadata_object);

        // mint the tokens and deposit the minted tokens to the `to` address
        let to_wallet = primary_fungible_store::ensure_primary_store_exists(to, metadata_object);
        let fiat_token = fungible_asset::mint(&fiat_token_config.mint_ref, amount);
        fungible_asset::deposit_with_ref(&fiat_token_config.transfer_ref, to_wallet, fiat_token);

        // decrease minter allowance by amount
        decrement_minter_allowance(signer::address_of(minter), amount);

        // TODO: emit minting event (??) or is FA events sufficient, indexing could be done that way
    }

    /// Burn fiat tokens from the minter's primary store.
    /// Returns true if operation is successful.
    public entry fun burn(minter: &signer, amount: u64) acquires FiatTokenConfig, FiatTokenAccessInfo {
        // SECURITY TODO: check if contract is paused 
        // SECURITY TODO: check if transaction authorizer (signer) is a minter OR owner

        // check if `minter` is blacklisted
        assert!(!is_blacklisted(signer::address_of(minter)), error::permission_denied(EBLACKLISTED_ADDRESS));

        // SECURITY TODO: check minter balance >= amount
        assert!(balance_of_primary_store(signer::address_of(minter)) < amount, error::permission_denied(EINSUFFICIENT_BALANCE));
        
        // retrieve the refs using the `Metadata` object
        let metadata_object = get_metadata_object();
        let fiat_token_config = authorized_borrow_refs(minter, metadata_object);

        // burn the tokens from the minter's primary store
        let from_wallet = primary_fungible_store::primary_store(signer::address_of(minter), metadata_object);
        fungible_asset::burn_from(&fiat_token_config.burn_ref, from_wallet, amount);

        // TODO: emit burning event (??) or is FA events sufficient, indexing could be done that way. any other events?
    }
    
    /// Transfer fiat tokens from `from` address to `to` address.
    /// Authorizer can be any signer with sufficient balance in their primary store.
    public entry fun transfer(from: &signer, to: address, amount: u64) acquires FiatTokenAccessInfo {
        // SECURITY TODO: check if contract is paused

        // check if `from` or  `to` address is blacklisted
        assert!(!is_blacklisted(signer::address_of(from)), error::permission_denied(EBLACKLISTED_ADDRESS));
        assert!(!is_blacklisted(to), error::permission_denied(EBLACKLISTED_ADDRESS));

        // check if amount is greater than `from` address's balance
        let from_addr = signer::address_of(from); 
        assert!(balance_of_primary_store(from_addr) < amount, error::permission_denied(EINSUFFICIENT_BALANCE));

        // transfer the tokens from `from` address to `to` address using transfer ref
        // transfer() with calls withdraw() and deposit(), emitting two events respectively
        let metadata_object = get_metadata_object();
        let from_wallet = primary_fungible_store::primary_store(from_addr, metadata_object);
        let to_wallet = primary_fungible_store::ensure_primary_store_exists(to, metadata_object);
        fungible_asset::transfer(from, from_wallet, to_wallet, amount);

        // TODO: do we need to emit an event here?
    }
    
    /// Add a new minter with an allowance or update the minter's allowance.
    /// Only callable by the master minter.
    /// Returns true if operation is successful.
    public(friend) fun configure_minter(master_minter: &signer, minter: address, allowance: u64): bool acquires MasterMinterConfig, FiatTokenAccessInfo {
        // SECURITY TODO: check if contract is paused

        // SECURITY TODO: check if caller is masterMinter
        let caller_addr = signer::address_of(master_minter);
        let master_minter_addr = borrow_global<MasterMinterConfig>(@circle).master_minter;
        assert!(caller_addr == master_minter_addr, error::permission_denied(ENOT_MASTER_MINTER));

        // SECURITY TODO: check if minter is blacklisted
        assert!(!is_blacklisted(minter), error::permission_denied(EBLACKLISTED_ADDRESS));

        // retrieve the access info and add a new minter and allowance OR update the minter's allowance if already exists
        let access_info = borrow_global_mut<FiatTokenAccessInfo>(@circle);
        smart_table::upsert(&mut access_info.minters, minter, allowance);

        // TODO: emit event
        true
    }
    
    /// Remove a minter from the minter's list.
    /// Only callable by the master minter.
    /// Returns true if operation is successful.
    public(friend) fun remove_minter(master_minter: &signer, minter: address): bool acquires MasterMinterConfig, FiatTokenAccessInfo {
        // SECURITY TODO: check if contract is paused

        // SECURITY TODO: check if caller is masterMinter 
        let caller_addr = signer::address_of(master_minter);
        let master_minter_addr = borrow_global<MasterMinterConfig>(@circle).master_minter;
        assert!(caller_addr == master_minter_addr, error::permission_denied(ENOT_MASTER_MINTER));

        // remove the minter from the minter's list
        let access_info = borrow_global_mut<FiatTokenAccessInfo>(@circle);
        smart_table::remove(&mut access_info.minters, minter);

        // TODO: emit event
        true
    }

    /// Updates the master minter address.
    /// Only callable by the owner/circle address.
    public entry fun update_master_minter(owner: &signer, new_master_minter: address) acquires MasterMinterConfig {
        // SECURITY TODO: check if contract is paused

        // SECURITY TODO: check if caller is owner
        assert!(signer::address_of(owner) == @circle, error::permission_denied(ENOT_OWNER));

        // update the master minter
        let master_minter_config = borrow_global_mut<MasterMinterConfig>(@circle);
        master_minter_config.master_minter = new_master_minter;

        // TODO: emit event
    }

    /// If signer is has the minter role, borrows the immutable reference of the refs of `Metadata` object and returns it.
    /// This validates that the signer is the Metadata object's creator.
    /// TODO: add description of inline functions
    inline fun authorized_borrow_refs(
        minter: &signer,
        asset: Object<Metadata>,
    ): &FiatTokenConfig acquires FiatTokenConfig {
        assert!(is_minter(signer::address_of(minter)), error::permission_denied(ECALLER_NOT_MINTER));
        borrow_global<FiatTokenConfig>(object::object_address(&asset))
    }
    
    /// Incrememnt minter allowance.
    inline fun increment_minter_allowance(minter_addr: address, increment_amount: u64) acquires FiatTokenAccessInfo {
        let new_allowance = get_minter_allowance(minter_addr) + increment_amount;
        let access_info = borrow_global_mut<FiatTokenAccessInfo>(@circle);
        smart_table::upsert(&mut access_info.minters, minter_addr, new_allowance);
        // TODO: emit event
    }

    /// Decrement minter allowance.
    inline fun decrement_minter_allowance(minter_addr: address, decrement_amount: u64) acquires FiatTokenAccessInfo {
        let new_allowance = get_minter_allowance(minter_addr) - decrement_amount;
        let access_info = borrow_global_mut<FiatTokenAccessInfo>(@circle);
        smart_table::upsert(&mut access_info.minters, minter_addr, new_allowance);
        // TODO: emit event
    }

    #[view]
    /// Return the address of the managed fungible asset that's created when this module is deployed.
    public fun get_metadata_object(): Object<Metadata> {
        let asset_address = object::create_object_address(&@circle, ASSET_SYMBOL);
        object::address_to_object<Metadata>(asset_address)
    }

    #[view]
    /// Return true if the account is blacklisted.
    fun is_blacklisted(account_addr: address): bool acquires FiatTokenAccessInfo {
        vector::contains(&borrow_global<FiatTokenAccessInfo>(@circle).blacklist, &account_addr)
    }

    #[view]
    /// Return true if caller is a minter.
    public fun is_minter(minter_addr: address): bool acquires FiatTokenAccessInfo {
        smart_table::contains(&borrow_global<FiatTokenAccessInfo>(@circle).minters, minter_addr)
    }

    #[view]
    /// Return the minter allowance for the given minter.
    public fun get_minter_allowance(minter_addr: address): u64 acquires FiatTokenAccessInfo {
        *smart_table::borrow(&borrow_global<FiatTokenAccessInfo>(@circle).minters, minter_addr)
    }

    #[view]
    /// Return the balance of the given account's primary fungible store.
    public fun balance_of_primary_store(account_addr: address): u64 {
        let metadata_object = get_metadata_object();
        primary_fungible_store::balance(account_addr, metadata_object)
    }

    /// view functions: totalSupply
    
    #[test(owner = @circle)]
    #[expected_failure]
    /// Test if the module is initialized successfully and verify it can only be called once.
    fun test_initialization(owner: &signer) acquires ModuleConfig {
        // check if module is already initialized
        let master_minter = signer::address_of(owner);

        // call initialize function
        initialize(
            owner,
            master_minter,
            1000000000,
            string::utf8(b"USD Coin"),
            string::utf8(b"USDC"),
            6,
        );
        assert!(borrow_global<ModuleConfig>(@circle).is_initialized, 1);

        // try to initialize the module again
        initialize(
            owner,
            master_minter,
            1000000000,
            string::utf8(b"USD Coin"),
            string::utf8(b"USDC"),
            6,
        );
    }

    #[test(owner = @circle, minter = @0x1, master_minter = @0x2)]
    #[expected_failure]
    /// Test if the minter can mint tokens successfully and verify the minter's allowance is decreased.
    /// Also, test if the minter can mint more than the allowance.
    fun test_mint(owner: &signer, minter: &signer, master_minter: &signer) acquires MasterMinterConfig, FiatTokenConfig, FiatTokenAccessInfo {
        let recipient = @0x3;
        // Setup initial state
        initialize(
            owner,
            signer::address_of(master_minter),
            1000000000,
            string::utf8(b"USD Coin"),
            string::utf8(b"USDC"),
            6,
        );

        // Set up minter allowance 
        configure_minter(master_minter, signer::address_of(minter), 1000);

        // Mint tokens to the recipient
        let amount_to_mint: u64 = 500;
        mint(minter, recipient, amount_to_mint);

        // Verify that the minter's allowance has decreased by the minted amount
        let minter_allowance = get_minter_allowance(signer::address_of(minter));
        assert!(minter_allowance == 500, 4); // Initial allowance (1000) - minted amount (500)
        let recipient_balance = balance_of_primary_store(recipient);
        assert!(recipient_balance == 500, 5); // Minted amount

        // Attempt to mint more than the allowance
        let excess_amount: u64 = 600;   
        mint(minter, recipient, excess_amount);
    }

    #[test(owner = @circle, minter = @0x1, master_minter = @0x2)]
    #[expected_failure]
    /// Test if the minter can burn tokens successfully.
    /// Also, test if the minter can burn more than the balance.
    fun test_burn(owner: &signer, minter: &signer, master_minter: &signer) acquires MasterMinterConfig, FiatTokenConfig, FiatTokenAccessInfo {
        // Setup initial state
        initialize(
            owner,
            signer::address_of(master_minter),
            1000000000,
            string::utf8(b"USD Coin"),
            string::utf8(b"USDC"),
            6,
        );

        // Set up minter allowance 
        configure_minter(master_minter, signer::address_of(minter), 1000);

        // Mint tokens to the minter
        let amount_to_mint: u64 = 500;
        mint(minter, signer::address_of(minter), amount_to_mint);

        // Burn tokens from the minter
        let amount_to_burn: u64 = 300;
        burn(minter, amount_to_burn);

        // Verify that the minter's balance has decreased by the burned amount
        let minter_balance = balance_of_primary_store(signer::address_of(minter));
        assert!(minter_balance == 200, 5); // Initial balance (500) - burned amount (300)

        // Attempt to burn more than the balance
        let excess_amount: u64 = 300;   
        burn(minter, excess_amount);
    }

    /// THINGS TO TEST REMAINING
    /// 





}

