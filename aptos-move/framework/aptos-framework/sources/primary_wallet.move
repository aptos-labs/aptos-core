/// This defines the module for interacting with primary wallets of accounts/objects, which have deterministic addresses
module aptos_framework::primary_wallet {
    use aptos_framework::create_signer;
    use aptos_framework::fungible_asset::{Self, FungibleAsset, FungibleAssetMetadata, FungibleAssetWallet};
    use aptos_framework::object::{Self, Object, ConstructorRef, DeriveRef};

    use std::signer;

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Resource stored on the fungible asset metadata object to allow creating primary wallets for it.
    struct PrimaryWalletSupport has key {
        metadata_derive_ref: DeriveRef,
    }

    /// Creators of fungible assets can call this to enable support for creating primary (deterministic) wallets for
    /// their users.
    public fun enable_primary_wallet(metadata_constructor_ref: &ConstructorRef) {
        // Ensure that this is a fungible asset metadata object.
        object::object_from_constructor_ref<FungibleAssetMetadata>(metadata_constructor_ref);
        let metadata_obj = &object::generate_signer(metadata_constructor_ref);
        move_to(metadata_obj, PrimaryWalletSupport {
            metadata_derive_ref: object::generate_derive_ref(metadata_constructor_ref),
        });
    }

    public fun ensure_primary_wallet_exists<T: key>(
        owner: address,
        metadata: Object<T>,
    ): Object<FungibleAssetWallet> acquires PrimaryWalletSupport {
        if (!primary_wallet_exists(owner, metadata)) {
            create_primary_wallet(owner, metadata);
        };
        primary_wallet(owner, metadata)
    }

    /// Create a primary wallet object to hold fungible asset for the given address.
    public fun create_primary_wallet<T: key>(
        owner_addr: address,
        metadata: Object<T>,
    ): Object<FungibleAssetWallet> acquires PrimaryWalletSupport {
        let owner = &create_signer::create_signer(owner_addr);
        let metadata_addr = object::object_address(&metadata);
        let derive_ref = &borrow_global<PrimaryWalletSupport>(metadata_addr).metadata_derive_ref;
        let constructor_ref = &object::create_derived_object(owner, derive_ref);

        // Disable ungated transfer as deterministic wallets shouldn't be transferrable.
        let transfer_ref = &object::generate_transfer_ref(constructor_ref);
        object::disable_ungated_transfer(transfer_ref);

        fungible_asset::create_wallet(constructor_ref, metadata)
    }

    #[view]
    public fun primary_wallet_address<T: key>(owner: address, metadata: Object<T>): address {
        let metadata_addr = object::object_address(&metadata);
        object::create_derived_object_address(owner, metadata_addr)
    }

    #[view]
    public fun primary_wallet<T: key>(owner: address, metadata: Object<T>): Object<FungibleAssetWallet> {
        let wallet = primary_wallet_address(owner, metadata);
        object::address_to_object<FungibleAssetWallet>(wallet)
    }

    #[view]
    public fun primary_wallet_exists<T: key>(account: address, metadata: Object<T>): bool {
        fungible_asset::wallet_exists(primary_wallet_address(account, metadata))
    }

    #[view]
    /// Get the balance of `account`'s primary wallet.
    public fun balance<T: key>(account: address, metadata: Object<T>): u64 {
        if (primary_wallet_exists(account, metadata)) {
            fungible_asset::balance(primary_wallet(account, metadata))
        } else {
            0
        }
    }

    #[view]
    /// Return whether the given account's primary wallet can do direct transfers.
    public fun ungated_transfer_allowed<T: key>(account: address, metadata: Object<T>): bool {
        fungible_asset::ungated_transfer_allowed(primary_wallet(account, metadata))
    }

    /// Withdraw `amount` of fungible asset from `wallet` by the owner.
    public fun withdraw<T: key>(owner: &signer, metadata: Object<T>, amount: u64): FungibleAsset {
        let wallet = primary_wallet(signer::address_of(owner), metadata);
        fungible_asset::withdraw(owner, wallet, amount)
    }

    /// Deposit `amount` of fungible asset to the given account's primary wallet.
    public fun deposit(owner: address, fa: FungibleAsset) acquires PrimaryWalletSupport {
        let metadata = fungible_asset::asset_metadata(&fa);
        let wallet = ensure_primary_wallet_exists(owner, metadata);
        fungible_asset::deposit(wallet, fa);
    }

    /// Transfer `amount` of fungible asset from sender's primary wallet to receiver's primary wallet.
    public entry fun transfer<T: key>(
        sender: &signer,
        metadata: Object<T>,
        recipient: address,
        amount: u64,
    ) acquires PrimaryWalletSupport {
        let sender_wallet = ensure_primary_wallet_exists(signer::address_of(sender), metadata);
        let recipient_wallet = ensure_primary_wallet_exists(recipient, metadata);
        fungible_asset::transfer(sender, sender_wallet, recipient_wallet, amount);
    }
}
