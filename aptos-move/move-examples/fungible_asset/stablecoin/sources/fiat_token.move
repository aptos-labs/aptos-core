module stablecoin_provider::fiat_token {
    use aptos_framework::fungible_asset::{Self, MintRef, TransferRef, BurnRef, Metadata};
    use aptos_framework::object::{Self, Object};
    use aptos_framework::primary_fungible_store;
    use std::error;
    use std::string::{Self, String};
    use std::option;
    use std::signer;
     use aptos_framework::event;
    use aptos_std::simple_map::{Self, SimpleMap};
    use aptos_std::smart_vector::{Self, SmartVector};


    /// Initialize function call only be called once.  
    const EALREADY_INITIALIZED: u64 = 1;
    /// Cannot perform operation on blacklisted address.
    const EBLACKLISTED_ADDRESS: u64 = 2;
    /// Caller is not a minter.
    const ECALLER_NOT_MINTER: u64 = 3;
    /// Minter allowance exceeded.
    const EMINTER_ALLOWANCE_EXCEEDED: u64 = 4;
    /// Insufficient balance.
    const EINSUFFICIENT_BALANCE: u64 = 5;
    /// Caller is not the master minter.
    const ENOT_MASTER_MINTER: u64 = 6;
    /// Caller is not the owner.
    const ENOT_OWNER: u64 = 7;
    /// Caller is not the blacklister.
    const ENOT_BLACKLISTER: u64 = 8;

    /// Asset Symbol
    const ASSET_SYMBOL: vector<u8> = b"Stablecoin Example";

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Store refs in `FA Metadata` object 
    struct FiatTokenConfig has key {
        mint_ref: MintRef,
        burn_ref: BurnRef,
        transfer_ref: TransferRef,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Store access info in `FA Metadata` object 
    struct FiatTokenAccessInfo has key {
        minters: SimpleMap<address, u64>,
        blacklist: SmartVector<address>,
        owner: address,
        master_minter: address,
        blacklister: address,
        pauser: address,
    }

    #[event]
    /// Emitted when new fiat tokens are minted.
    struct Mint has drop, store {
        minter: address,
        to: address,
        amount: u64,
    }

    #[event]
    /// Emitted when fiat tokens are burned.
    struct Burn has drop, store {
        burner: address,
        amount: u64,
    }

    #[event]
    /// Emitted when a new minter is configured or an existing minter's allowance is updated.
    struct MinterConfigured has drop, store {
        minter: address,
        allowance: u64,
    }

    #[event]
    /// Emitted when a minter is removed.
    struct MinterRemoved has drop, store {
        old_minter: address,
    }

    #[event]
    /// Emitted when the master minter is changed.
    struct MasterMinterChanged has drop, store {
        new_master_minter: address,
    }


    /// Initialize metadata object and store it in the `Metadata` resource
    /// This function is called only once when the module is deployed. 
    /// If called twice, it will fail since `create_named_object` cannot be called with the same seed twice. 
    /// Only accessible by the REST API
    entry fun initialize(
        creator: &signer,
        maximum_supply: u128,
        name: String,
        symbol: String,
        decimals: u8,
        owner: address,
        master_minter: address,
        pauser: address,
        blacklister: address,
    ) {
        // TODO: check if module is already initialized
        

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
        // TODO: need to specify that only untransferable stores can be created for this FA
        let constructor_ref = &object::create_named_object(creator, ASSET_SYMBOL);
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

        // store access info in `FiatTokenAccessInfo` resource, move resource to `Metadata object` address
        move_to(
            &metadata_object_signer,
            FiatTokenAccessInfo {
                minters: simple_map::new(),
                blacklist: smart_vector::empty(),
                owner,
                master_minter,
                blacklister,
                pauser,
            },
        );
    }
    
    /// Mint new fiat tokens and deposit them to the `to` address.
    /// Returns true if operation is successful.
    public entry fun mint(minter: &signer, to: address, amount: u64) acquires FiatTokenConfig, FiatTokenAccessInfo  {
        // SECURITY TODO: check if contract is paused 

        // check if transaction authorizer (signer) is a minter 
        assert!(is_minter(signer::address_of(minter)), error::permission_denied(ECALLER_NOT_MINTER));

        // check if `minter` or `to` address is blacklisted
        assert!(!is_blacklisted(signer::address_of(minter)), error::permission_denied(EBLACKLISTED_ADDRESS));
        assert!(!is_blacklisted(to), error::permission_denied(EBLACKLISTED_ADDRESS));
        
        // check if minter allowance >= amount 
        assert!(get_minter_allowance(signer::address_of(minter)) >= amount, error::invalid_argument(EMINTER_ALLOWANCE_EXCEEDED));
        
        // retrieve the refs using the `Metadata` object
        let metadata_object = get_metadata_object();
        let fiat_token_config = authorized_borrow_refs(minter, metadata_object);

        // mint the tokens and deposit the minted tokens to the `to` address
        let to_wallet = primary_fungible_store::ensure_primary_store_exists(to, metadata_object);
        let fiat_token = fungible_asset::mint(&fiat_token_config.mint_ref, amount);

        // emit Mint event
        event::emit(Mint { minter: signer::address_of(minter), to, amount });
        
        // FA module emits deposit event
        fungible_asset::deposit_with_ref(&fiat_token_config.transfer_ref, to_wallet, fiat_token);

        // decrease minter allowance by amount
        decrement_minter_allowance(signer::address_of(minter), amount);
    }

    /// Burn fiat tokens from the minter's primary store.
    /// Returns true if operation is successful.
    public entry fun burn(minter: &signer, amount: u64) acquires FiatTokenConfig, FiatTokenAccessInfo {
        // SECURITY TODO: check if contract is paused 

        // check if transaction authorizer (signer) is a minter OR owner
        assert!(is_minter(signer::address_of(minter)), error::permission_denied(ECALLER_NOT_MINTER));
        
        // check if `minter` is blacklisted
        assert!(!is_blacklisted(signer::address_of(minter)), error::permission_denied(EBLACKLISTED_ADDRESS));
        
        // retrieve the refs using the `Metadata` object
        let metadata_object = get_metadata_object();
        let fiat_token_config = authorized_borrow_refs(minter, metadata_object);

        // burn the tokens from the minter's primary store
        let from_wallet = primary_fungible_store::primary_store(signer::address_of(minter), metadata_object);

        // note `burn_from` will call `withdraw_internal` on FA module and check if the `from_wallet` has sufficient balance 
        fungible_asset::burn_from(&fiat_token_config.burn_ref, from_wallet, amount);

        // emit Burn event
        event::emit(Burn { burner: signer::address_of(minter), amount });
    }
    
    /// Add a new minter with an allowance or update the minter's allowance.
    /// Only callable by the master minter.
    public fun configure_minter(master_minter: &signer, minter: address, allowance: u64) acquires FiatTokenAccessInfo {
        // SECURITY TODO: check if contract is paused

        // check if minter is blacklisted
        assert!(!is_blacklisted(minter), error::permission_denied(EBLACKLISTED_ADDRESS));

        // check if caller is masterMinter
        let caller_addr = signer::address_of(master_minter);
        let metadata_object = get_metadata_object();
        let access_info = borrow_access_info_mut(metadata_object);
        assert!(caller_addr == access_info.master_minter, error::permission_denied(ENOT_MASTER_MINTER));

        // add a new minter and allowance OR update the minter's allowance if already exists
        simple_map::upsert(&mut access_info.minters, minter, allowance);

        // emit event
        event::emit(MinterConfigured { minter, allowance });
    }
    
    /// Remove a minter from the minter's list.
    /// Only callable by the master minter.
    public fun remove_minter(master_minter: &signer, minter: address) acquires FiatTokenAccessInfo {
        // SECURITY TODO: check if contract is paused

        // check if caller is masterMinter 
        let caller_addr = signer::address_of(master_minter);
        let metadata_object = get_metadata_object();
        let access_info = borrow_access_info_mut(metadata_object);
        assert!(caller_addr == access_info.master_minter, error::permission_denied(ENOT_MASTER_MINTER));

        // remove the minter from the minter's list
        simple_map::remove(&mut access_info.minters, &minter);

        // emit event
        event::emit(MinterRemoved { old_minter: minter });
    }

    /// Blacklist an address.
    /// Only callable by the blacklister address.
    public entry fun blacklist(blacklister: &signer, account_addr: address) acquires FiatTokenAccessInfo {
        // SECURITY TODO: check if contract is paused

        // check if signer is blacklister
        let metadata_object = get_metadata_object();
        let access_info = borrow_access_info_mut(metadata_object);
        assert!(signer::address_of(blacklister) == access_info.blacklister, error::permission_denied(ENOT_BLACKLISTER));

        // retrieve the transfer_ref and freeze the account's primary store
        let fiat_token_config = authorized_borrow_refs(blacklister, metadata_object).transfer_ref;
        primary_fungible_store::set_frozen_flag(transfer_ref, account_addr, true);
    }

    /// Updates the master minter address.
    /// Only callable by the owner/stablecoin provider address.
    public entry fun update_master_minter(owner: &signer, new_master_minter: address) acquires FiatTokenAccessInfo {
        // SECURITY TODO: check if contract is paused

        // check if caller is owner
        assert!(signer::address_of(owner) == @stablecoin_provider, error::permission_denied(ENOT_OWNER));

        // update the master minter
        let metadata_object = get_metadata_object();
        let access_info = borrow_access_info_mut(metadata_object);
        access_info.master_minter = new_master_minter;

        // emit event
        event::emit(MasterMinterChanged { new_master_minter });
    }

    /// Updates the blacklister address.
    /// Only callable by the owner/stablecoin provider address.
    public entry fun update_blacklister(owner: &signer, new_blacklister: address) acquires FiatTokenAccessInfo {
        // SECURITY TODO: check if contract is paused

        // check if caller is owner
        assert!(signer::address_of(owner) == @stablecoin_provider, error::permission_denied(ENOT_OWNER));

        // update the blacklister
        let metadata_object = get_metadata_object();
        let access_info = borrow_access_info_mut(metadata_object);
        access_info.blacklister = new_blacklister;
    }

    /// If signer is has the minter/blacklister role, borrows the immutable reference of the refs of `Metadata` object and returns it.
    /// This validates that the signer is the Metadata object's creator.
    inline fun authorized_borrow_refs(
        caller: &signer,
        asset: Object<Metadata>,
    ): &FiatTokenConfig acquires FiatTokenConfig {
        // TODO: check either minter or blacklister
        assert!(is_minter(signer::address_of(caller)), error::permission_denied(ECALLER_NOT_MINTER));
        borrow_global<FiatTokenConfig>(object::object_address(&asset))
    }

    /// Borrows the mutable reference of the refs of `Metadata` object and returns it.
    inline fun borrow_access_info_mut(
        asset: Object<Metadata>,
    ): &mut FiatTokenAccessInfo acquires FiatTokenAccessInfo {
        borrow_global_mut<FiatTokenAccessInfo>(object::object_address(&asset))
    }

    /// Decrement minter allowance.
    inline fun decrement_minter_allowance(minter_addr: address, decrement_amount: u64) acquires FiatTokenAccessInfo {
        let new_allowance = get_minter_allowance(minter_addr) - decrement_amount;
        let metadata_object = get_metadata_object();
        let access_info = borrow_access_info_mut(metadata_object);

        // upsert() will insert a new key-value pair if the key does not exist, otherwise it will update the value of the existing key
        simple_map::upsert(&mut access_info.minters, minter_addr, new_allowance);
        // TODO: emit event
    }

    #[view]
    /// Return the address of the managed fungible asset that's created when this module is deployed.
    public fun get_metadata_object(): Object<Metadata> {
        let asset_address = object::create_object_address(&@stablecoin_provider, ASSET_SYMBOL);
        object::address_to_object<Metadata>(asset_address)
    }

    #[view]
    /// Return true if the account is blacklisted.
    /// TODO: we shouldn't have a centralized blacklist, use the freeze functions of primary stores
    fun is_blacklisted(account_addr: address): bool acquires FiatTokenAccessInfo {
        // get the account's primary store object, and check if the store is frozen
        let wallet = primary_fungible_store::primary_store(account_addr, get_metadata_object());
        primary_fungible_store::is_frozen(wallet)
    }

    #[view]
    /// Return true if caller is a minter.
    public fun is_minter(minter_addr: address): bool acquires FiatTokenAccessInfo {
        let asset = get_metadata_object();
        simple_map::contains_key(&borrow_global<FiatTokenAccessInfo>(object::object_address(&asset)).minters, &minter_addr)
    }

    #[view]
    /// Return the minter allowance for the given minter.
    public fun get_minter_allowance(minter_addr: address): u64 acquires FiatTokenAccessInfo {
        let asset = get_metadata_object();
        *simple_map::borrow(&borrow_global<FiatTokenAccessInfo>(object::object_address(&asset)).minters, &minter_addr)
    }

    #[view]
    /// Return the balance of the given account's primary fungible store.
    public fun balance_of_primary_store(account_addr: address): u64 {
        let metadata_object = get_metadata_object();
        primary_fungible_store::balance(account_addr, metadata_object)
    }
    
    // view function: totalsupply

    // #[test(owner = @stablecoin_provider)]
    // #[expected_failure]
    // /// Test if the module is initialized successfully and verify it can only be called once.
    // fun test_initialization(owner: &signer) acquires ModuleConfig {
    //     // check if module is already initialized
    //     let master_minter = signer::address_of(owner);

    //     // call initialize function
    //     initialize(
    //         owner,
    //         master_minter,
    //         1000000000,
    //         string::utf8(b"USD Coin"),
    //         string::utf8(b"USDC"),
    //         6,
    //     );
    //     assert!(borrow_global<ModuleConfig>(@stablecoin_provider).is_initialized, 1);

    //     // try to initialize the module again
    //     initialize(
    //         owner,
    //         master_minter,
    //         1000000000,
    //         string::utf8(b"USD Coin"),
    //         string::utf8(b"USDC"),
    //         6,
    //     );
    // }

    // #[test(owner = @stablecoin_provider, minter = @0x1, master_minter = @0x2)]
    // #[expected_failure]
    // /// Test if the minter can mint tokens successfully and verify the minter's allowance is decreased.
    // /// Also, test if the minter can mint more than the allowance.
    // fun test_mint(owner: &signer, minter: &signer, master_minter: &signer) acquires FiatTokenConfig, FiatTokenAccessInfo {
    //     let recipient = @0x3;
    //     // Setup initial state
    //     initialize(
    //         owner,
    //         signer::address_of(master_minter),
    //         1000000000,
    //         string::utf8(b"USD Coin"),
    //         string::utf8(b"USDC"),
    //         6,
    //     );

    //     // Set up minter allowance 
    //     configure_minter(master_minter, signer::address_of(minter), 1000);

    //     // Mint tokens to the recipient
    //     let amount_to_mint: u64 = 500;
    //     mint(minter, recipient, amount_to_mint);

    //     // Verify that the minter's allowance has decreased by the minted amount
    //     let minter_allowance = get_minter_allowance(signer::address_of(minter));
    //     assert!(minter_allowance == 500, 4); // Initial allowance (1000) - minted amount (500)
    //     let recipient_balance = balance_of_primary_store(recipient);
    //     assert!(recipient_balance == 500, 5); // Minted amount

    //     // Attempt to mint more than the allowance
    //     let excess_amount: u64 = 600;   
    //     mint(minter, recipient, excess_amount);
    // }

    // #[test(owner = @stablecoin_provider, minter = @0x1, master_minter = @0x2)]
    // #[expected_failure]
    // /// Test if the minter can burn tokens successfully.
    // /// Also, test if the minter can burn more than the balance.
    // fun test_burn(owner: &signer, minter: &signer, master_minter: &signer) acquires FiatTokenConfig, FiatTokenAccessInfo {
    //     // Setup initial state
    //     initialize(
    //         owner,
    //         signer::address_of(master_minter),
    //         1000000000,
    //         string::utf8(b"USD Coin"),
    //         string::utf8(b"USDC"),
    //         6,
    //     );

    //     // Set up minter allowance 
    //     configure_minter(master_minter, signer::address_of(minter), 1000);

    //     // Mint tokens to the minter
    //     let amount_to_mint: u64 = 500;
    //     mint(minter, signer::address_of(minter), amount_to_mint);

    //     // Burn tokens from the minter
    //     let amount_to_burn: u64 = 300;
    //     burn(minter, amount_to_burn);

    //     // Verify that the minter's balance has decreased by the burned amount
    //     let minter_balance = balance_of_primary_store(signer::address_of(minter));
    //     assert!(minter_balance == 200, 5); // Initial balance (500) - burned amount (300)

    //     // Attempt to burn more than the balance
    //     let excess_amount: u64 = 300;   
    //     burn(minter, excess_amount);
    // }

    // // THINGS TO TEST REMAINING
}

