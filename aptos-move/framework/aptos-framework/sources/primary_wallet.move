/// This defines the module for interacting with primary wallets of accounts/objects, which have deterministic addresses
module aptos_framework::primary_wallet {
    use aptos_framework::fungible_asset::{Self, FungibleAsset, FungibleAssetWallet};
    use aptos_framework::object::{Self, Object};

    use std::signer;

    public fun ensure_primary_wallet_exists<T: key>(owner: address, metadata: Object<T>): Object<FungibleAssetWallet> {
        if (!primary_wallet_exists(owner, metadata)) {
            create_primary_wallet(owner, metadata);
        };
        primary_wallet(owner, metadata)
    }

    /// Create a primary wallet object to hold fungible asset for the given address.
    public fun create_primary_wallet<T: key>(owner: address, metadata: Object<T>): Object<FungibleAssetWallet> {
        fungible_asset::create_deterministic_wallet(owner, metadata)
    }

    #[view]
    public fun primary_wallet_address<T: key>(owner: address, metadata: Object<T>): address {
        fungible_asset::deterministic_wallet_address(owner, metadata)
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
    public fun deposit(owner: address, fa: FungibleAsset) {
        let metadata = fungible_asset::asset_metadata(&fa);
        let wallet = ensure_primary_wallet_exists(owner, metadata);
        fungible_asset::deposit(wallet, fa);
    }

    /// Transfer `amount` of fungible asset from sender's primary wallet to receiver's primary wallet.
    public entry fun transfer<T: key>(sender: &signer, metadata: Object<T>, amount: u64, recipient: address) {
        let sender_wallet = ensure_primary_wallet_exists(signer::address_of(sender), metadata);
        let recipient_wallet = ensure_primary_wallet_exists(recipient, metadata);
        fungible_asset::transfer(sender, sender_wallet, amount, recipient_wallet);
    }
}
