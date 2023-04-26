/// This defines the module for interacting with primary stores of accounts/objects, which have deterministic addresses
module aptos_framework::primary_fungible_store {
    use aptos_framework::fungible_asset::{Self, FungibleAsset, FungibleStore};
    use aptos_framework::object::{Self, Object, ConstructorRef, DeriveRef};

    use std::option::Option;
    use std::signer;
    use std::string::String;

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Resource stored on the fungible asset metadata object to allow creating primary stores for it.
    struct DeriveRefPod has key {
        metadata_derive_ref: DeriveRef,
    }

    /// Creators of fungible assets can call this to enable support for creating primary (deterministic) stores for
    /// their users.
    public fun create_primary_store_enabled_fungible_asset(
        constructor_ref: &ConstructorRef,
        monitoring_supply_with_maximum: Option<Option<u128>>,
        name: String,
        symbol: String,
        decimals: u8,
    ) {
        fungible_asset::add_fungibility(constructor_ref, monitoring_supply_with_maximum, name, symbol, decimals);
        let metadata_obj = &object::generate_signer(constructor_ref);
        move_to(metadata_obj, DeriveRefPod {
            metadata_derive_ref: object::generate_derive_ref(constructor_ref),
        });
    }

    public fun ensure_primary_store_exists<T: key>(
        owner: address,
        metadata: Object<T>,
    ): Object<FungibleStore> acquires DeriveRefPod {
        if (!primary_store_exists(owner, metadata)) {
            create_primary_store(owner, metadata)
        } else {
            primary_store(owner, metadata)
        }
    }

    /// Create a primary store object to hold fungible asset for the given address.
    public fun create_primary_store<T: key>(
        owner_addr: address,
        metadata: Object<T>,
    ): Object<FungibleStore> acquires DeriveRefPod {
        let metadata_addr = object::object_address(&metadata);
        let derive_ref = &borrow_global<DeriveRefPod>(metadata_addr).metadata_derive_ref;
        let constructor_ref = &object::create_user_derived_object(owner_addr, derive_ref);

        // Disable ungated transfer as deterministic stores shouldn't be transferrable.
        let transfer_ref = &object::generate_transfer_ref(constructor_ref);
        object::disable_ungated_transfer(transfer_ref);

        fungible_asset::create_store(constructor_ref, metadata)
    }

    #[view]
    public fun primary_store_address<T: key>(owner: address, metadata: Object<T>): address {
        let metadata_addr = object::object_address(&metadata);
        object::create_user_derived_object_address(owner, metadata_addr)
    }

    #[view]
    public fun primary_store<T: key>(owner: address, metadata: Object<T>): Object<FungibleStore> {
        let store = primary_store_address(owner, metadata);
        object::address_to_object<FungibleStore>(store)
    }

    #[view]
    public fun primary_store_exists<T: key>(account: address, metadata: Object<T>): bool {
        fungible_asset::store_exists(primary_store_address(account, metadata))
    }

    #[view]
    /// Get the balance of `account`'s primary store.
    public fun balance<T: key>(account: address, metadata: Object<T>): u64 {
        if (primary_store_exists(account, metadata)) {
            fungible_asset::balance(primary_store(account, metadata))
        } else {
            0
        }
    }

    #[view]
    /// Return whether the given account's primary store is frozen.
    public fun is_frozen<T: key>(account: address, metadata: Object<T>): bool {
        if (primary_store_exists(account, metadata)) {
            fungible_asset::is_frozen(primary_store(account, metadata))
        } else {
            false
        }
    }

    /// Withdraw `amount` of fungible asset from `store` by the owner.
    public fun withdraw<T: key>(owner: &signer, metadata: Object<T>, amount: u64): FungibleAsset {
        let store = primary_store(signer::address_of(owner), metadata);
        fungible_asset::withdraw(owner, store, amount)
    }

    /// Deposit `amount` of fungible asset to the given account's primary store.
    public fun deposit(owner: address, fa: FungibleAsset) acquires DeriveRefPod {
        let metadata = fungible_asset::asset_metadata(&fa);
        let store = ensure_primary_store_exists(owner, metadata);
        fungible_asset::deposit(store, fa);
    }

    /// Transfer `amount` of fungible asset from sender's primary store to receiver's primary store.
    public entry fun transfer<T: key>(
        sender: &signer,
        metadata: Object<T>,
        recipient: address,
        amount: u64,
    ) acquires DeriveRefPod {
        let sender_store = ensure_primary_store_exists(signer::address_of(sender), metadata);
        let recipient_store = ensure_primary_store_exists(recipient, metadata);
        fungible_asset::transfer(sender, sender_store, recipient_store, amount);
    }

    #[test_only]
    use aptos_framework::fungible_asset::{create_test_token, mint, generate_mint_ref, generate_burn_ref, MintRef, TransferRef, BurnRef, generate_transfer_ref};
    #[test_only]
    use std::string;
    #[test_only]
    use std::option;

    #[test_only]
    public fun init_test_metadata_with_primary_store_enabled(
        constructor_ref: &ConstructorRef
    ): (MintRef, TransferRef, BurnRef) {
        create_primary_store_enabled_fungible_asset(
            constructor_ref,
            option::some(option::some(100)) /* max supply */,
            string::utf8(b"USDA"),
            string::utf8(b"$$$"),
            0
        );
        let mint_ref = generate_mint_ref(constructor_ref);
        let burn_ref = generate_burn_ref(constructor_ref);
        let transfer_ref = generate_transfer_ref(constructor_ref);
        (mint_ref, transfer_ref, burn_ref)
    }

    #[test(creator = @0xcafe, aaron = @0xface)]
    fun test_default_behavior(creator: &signer, aaron: &signer) acquires DeriveRefPod {
        let (creator_ref, metadata) = create_test_token(creator);
        init_test_metadata_with_primary_store_enabled(&creator_ref);
        let creator_address = signer::address_of(creator);
        let aaron_address = signer::address_of(aaron);
        assert!(!primary_store_exists(creator_address, metadata), 1);
        assert!(!primary_store_exists(aaron_address, metadata), 2);
        assert!(balance(creator_address, metadata) == 0, 3);
        assert!(balance(aaron_address, metadata) == 0, 4);
        assert!(!is_frozen(creator_address, metadata), 5);
        assert!(!is_frozen(aaron_address, metadata), 6);
        ensure_primary_store_exists(creator_address, metadata);
        ensure_primary_store_exists(aaron_address, metadata);
        assert!(primary_store_exists(creator_address, metadata), 7);
        assert!(primary_store_exists(aaron_address, metadata), 8);
    }

    #[test(creator = @0xcafe, aaron = @0xface)]
    fun test_basic_flow(
        creator: &signer,
        aaron: &signer,
    ) acquires DeriveRefPod {
        let (creator_ref, metadata) = create_test_token(creator);
        let (mint_ref, _transfer_ref, _burn_ref) = init_test_metadata_with_primary_store_enabled(&creator_ref);
        let creator_address = signer::address_of(creator);
        let aaron_address = signer::address_of(aaron);
        assert!(balance(creator_address, metadata) == 0, 1);
        assert!(balance(aaron_address, metadata) == 0, 2);
        let fa = mint(&mint_ref, 100);
        deposit(creator_address, fa);
        transfer(creator, metadata, aaron_address, 80);
        let fa = withdraw(aaron, metadata, 10);
        deposit(creator_address, fa);
        assert!(balance(creator_address, metadata) == 30, 3);
        assert!(balance(aaron_address, metadata) == 70, 4);
    }
}
