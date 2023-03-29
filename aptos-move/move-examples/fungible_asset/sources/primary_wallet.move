/// This defines the module for interacting with primary wallets of accounts/objects, which have deterministic addresses
module fungible_asset::primary_wallet {
    use fungible_asset::fungible_asset::{Self, FungibleAsset, FungibleAssetMetadata, FungibleAssetWallet};
    use aptos_framework::object::{Self, Object};

    use std::signer;

    public entry fun create_primary_wallet_entry(owner: &signer, metadata: address) {
        fungible_asset::create_deterministic_wallet(owner, verify(metadata));
    }

    /// Create a primary wallet object to hold fungible asset for the given address.
    public fun create_primary_wallet(owner: &signer, metadata: address): Object<FungibleAssetWallet> {
        fungible_asset::create_deterministic_wallet(owner, verify(metadata))
    }

    inline fun verify(metadata: address): Object<FungibleAssetMetadata> {
        object::address_to_object<FungibleAssetMetadata>(metadata)
    }

    #[view]
    public fun primary_wallet_address(owner: address, metadata: address): address {
        fungible_asset::create_deterministic_wallet_address(owner, verify(metadata))
    }

    #[view]
    public fun primary_wallet_reference(owner: address, metadata: address): Object<FungibleAssetWallet> {
        let wallet = primary_wallet_address(owner, metadata);
        object::address_to_object<FungibleAssetWallet>(wallet)
    }

    #[view]
    public fun primary_wallet_exists(account: address, metadata: address): bool {
        fungible_asset::wallet_exists_at(primary_wallet_address(account, metadata))
    }

    #[view]
    /// Get the balance of `account`'s primary wallet.
    public fun balance(account: address, metadata: address): u64 {
        if (primary_wallet_exists(account, metadata)) {
            fungible_asset::balance(primary_wallet_reference(account, metadata))
        } else {
            0
        }
    }

    #[view]
    /// Return whether the given account's primary wallet can do direct transfers.
    public fun ungated_transfer_allowed(account: address, metadata: address): bool {
        fungible_asset::ungated_transfer_allowed(primary_wallet_reference(account, metadata))
    }

    /// Withdraw `amount` of fungible asset from `wallet` by the owner.
    public fun withdraw(owner: &signer, metadata: address, amount: u64): FungibleAsset {
        let wallet = primary_wallet_reference(signer::address_of(owner), metadata);
        fungible_asset::withdraw(owner, wallet, amount)
    }

    /// Deposit `amount` of fungible asset to the given account's primary wallet.
    public fun deposit(owner: address, fa: FungibleAsset) {
        let metadata = object::object_address(&fungible_asset::asset_metadata(&fa));
        let wallet = primary_wallet_reference(owner, metadata);
        fungible_asset::deposit(wallet, fa);
    }

    /// Transfer `amount` of fungible asset from sender's primary wallet to receiver's primary wallet.
    public entry fun transfer(sender: &signer, metadata: address, amount: u64, recipient: address) {
        let sender_wallet = primary_wallet_reference(signer::address_of(sender), metadata);
        let recipient_wallet = primary_wallet_reference(recipient, metadata);
        fungible_asset::transfer(sender, sender_wallet, amount, recipient_wallet);
    }
}
