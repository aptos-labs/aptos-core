/// This defines the module for interacting with primary stores of accounts/objects, which have deterministic addresses
module aptos_framework::primary_store {
    use aptos_framework::create_signer;
    use aptos_framework::fungible_asset::{Self, FungibleAsset, FungibleAssetStore};
    use aptos_framework::object::{Self, Object, ConstructorRef, DeriveRef};

    use std::signer;
    use std::string::String;

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Resource stored on the fungible asset metadata object to allow creating primary stores for it.
    struct DeriveRefPod has key {
        metadata_derive_ref: DeriveRef,
    }

    /// Creators of fungible assets can call this to enable support for creating primary (deterministic) stores for
    /// their users.
    public fun create_primary_wallet_enabled_fungible_asset(
        constructor_ref: &ConstructorRef,
        maximum_supply: u64,
        name: String,
        symbol: String,
        decimals: u8,
    ) {
        fungible_asset::add_fungibility(constructor_ref, maximum_supply, name, symbol, decimals);
        let metadata_obj = &object::generate_signer(constructor_ref);
        move_to(metadata_obj, DeriveRefPod {
            metadata_derive_ref: object::generate_derive_ref(constructor_ref),
        });
    }

    public fun ensure_primary_store_exists<T: key>(
        owner: address,
        metadata: Object<T>,
    ): Object<FungibleAssetStore> acquires DeriveRefPod {
        if (!primary_store_exists(owner, metadata)) {
            create_primary_store(owner, metadata);
        };
        primary_store(owner, metadata)
    }

    /// Create a primary store object to hold fungible asset for the given address.
    public fun create_primary_store<T: key>(
        owner_addr: address,
        metadata: Object<T>,
    ): Object<FungibleAssetStore> acquires DeriveRefPod {
        let owner = &create_signer::create_signer(owner_addr);
        let metadata_addr = object::object_address(&metadata);
        let derive_ref = &borrow_global<DeriveRefPod>(metadata_addr).metadata_derive_ref;
        let constructor_ref = &object::create_user_derived_object(owner, derive_ref);

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
    public fun primary_store<T: key>(owner: address, metadata: Object<T>): Object<FungibleAssetStore> {
        let store = primary_store_address(owner, metadata);
        object::address_to_object<FungibleAssetStore>(store)
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
    /// Return whether the given account's primary store can do direct transfers.
    public fun ungated_transfer_allowed<T: key>(account: address, metadata: Object<T>): bool {
        fungible_asset::ungated_transfer_allowed(primary_store(account, metadata))
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
}
