/// By deploying this module, the deployer will be creating a new managed fungible asset with the hardcoded
/// maximum supply, name, symbol, and decimals. The address of the asset can be obtained via get_asset().
/// The deployer will also become the initial admin and can mint/burn/freeze/unfreeze accounts.
/// The admin can transfer the asset via object::transfer() at any point to set a new admin.
module fungible_asset::managed_fungible_asset {
    use aptos_framework::object;
    use fungible_asset::fungible_asset::{Self, MintRef, TransferRef, BurnRef, FungibleAsset, FungibleAssetMetadata};
    use fungible_asset::primary_wallet;
    use std::error;
    use std::signer;
    use std::string::utf8;

    /// Only fungible asset metadata owner can make changes.
    const ENOT_OWNER: u64 = 1;

    const ASSET_SYMBOL: vector<u8> = b"APT";

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Hold refs to control the minting, transfer and burning of fungible assets.
    struct ManagedFungibleAsset has key {
        mint_ref: MintRef,
        transfer_ref: TransferRef,
        burn_ref: BurnRef,
    }

    /// Initialize metadata object and store the refs.
    fun init_module(admin: &signer) {
        let constructor_ref = &object::create_named_object(admin, ASSET_SYMBOL);
        let (mint_ref, transfer_ref, burn_ref) = fungible_asset::make_object_fungible(
            constructor_ref,
            0, /* maximum_supply. 0 means no maximum */
            utf8(b"Aptos Token"), /* name */
            utf8(ASSET_SYMBOL), /* symbol */
            8, /* decimals */
        );
        let metadata_object_signer = object::generate_signer(constructor_ref);
        move_to(
            &metadata_object_signer,
            ManagedFungibleAsset { mint_ref, transfer_ref, burn_ref }
        )
    }

    #[view]
    /// Return the address of the managed fungible asset that's created when this module is deployed.
    public fun get_asset(): address {
        object::create_object_address(&@fungible_asset, ASSET_SYMBOL)
    }

    /// Mint as the owner of metadata object.
    public entry fun mint(
        admin: &signer,
        metadata: address,
        amount: u64,
        to: address
    ) acquires ManagedFungibleAsset {
        let mint_ref = &authorized_borrow_refs(admin, metadata).mint_ref;
        let to_wallet = primary_wallet::primary_wallet_reference(to, metadata);
        fungible_asset::deposit(to_wallet, fungible_asset::mint(mint_ref, amount));
    }

    /// Transfer as the owner of metadata object ignoring `allow_ungated_transfer` field.
    public entry fun transfer(
        admin: &signer,
        metadata: address,
        from: address,
        to: address,
        amount: u64,
    ) acquires ManagedFungibleAsset {
        let transfer_ref = &authorized_borrow_refs(admin, metadata).transfer_ref;
        let from_wallet = primary_wallet::primary_wallet_reference(from, metadata);
        let to_wallet = primary_wallet::primary_wallet_reference(to, metadata);
        fungible_asset::transfer_with_ref(transfer_ref, from_wallet, amount, to_wallet);
    }

    /// Burn fungible assets as the owner of metadata object.
    public entry fun burn(
        admin: &signer,
        metadata: address,
        from: address,
        amount: u64
    ) acquires ManagedFungibleAsset {
        let burn_ref = &authorized_borrow_refs(admin, metadata).burn_ref;
        let from_wallet = primary_wallet::primary_wallet_reference(from, metadata);
        fungible_asset::burn(burn_ref, from_wallet, amount);
    }

    /// Freeze an account so it cannot transfer or receive fungible assets.
    public entry fun freeze_account<T: key>(
        admin: &signer,
        metadata: address,
        account: address,
    ) acquires ManagedFungibleAsset {
        let transfer_ref = &authorized_borrow_refs(admin, metadata).transfer_ref;
        let wallet = primary_wallet::primary_wallet_reference(account, metadata);
        fungible_asset::set_ungated_transfer(transfer_ref, wallet, false);
    }

    /// Unfreeze an account so it can transfer or receive fungible assets.
    public entry fun unfreeze_account(
        admin: &signer,
        metadata: address,
        account: address,
    ) acquires ManagedFungibleAsset {
        let transfer_ref = &authorized_borrow_refs(admin, metadata).transfer_ref;
        let wallet = primary_wallet::primary_wallet_reference(account, metadata);
        fungible_asset::set_ungated_transfer(transfer_ref, wallet, true);
    }

    /// Withdraw as the owner of metadata object ignoring `allow_ungated_transfer` field.
    public fun withdraw(
        admin: &signer,
        metadata: address,
        amount: u64,
        from: address,
    ): FungibleAsset acquires ManagedFungibleAsset {
        let transfer_ref = &authorized_borrow_refs(admin, metadata).transfer_ref;
        let from_wallet = primary_wallet::primary_wallet_reference(from, metadata);
        fungible_asset::withdraw_with_ref(transfer_ref, from_wallet, amount)
    }

    /// Deposit as the owner of metadata object ignoring `allow_ungated_transfer` field.
    public fun deposit(
        admin: &signer,
        metadata: address,
        to: address,
        fa: FungibleAsset
    ) acquires ManagedFungibleAsset {
        let transfer_ref = &authorized_borrow_refs(admin, metadata).transfer_ref;
        let to_wallet = primary_wallet::primary_wallet_reference(to, metadata);
        fungible_asset::deposit_with_ref(transfer_ref, to_wallet, fa);
    }

    /// Borrow the immutable reference of the refs of `metadata`.
    /// This validates that the signer is the metadata object's owner.
    inline fun authorized_borrow_refs(
        owner: &signer,
        metadata: address,
    ): &ManagedFungibleAsset acquires ManagedFungibleAsset {
        let metadata = object::address_to_object<FungibleAssetMetadata>(metadata);
        assert!(object::is_owner(metadata, signer::address_of(owner)), error::permission_denied(ENOT_OWNER));
        borrow_global<ManagedFungibleAsset>(object::object_address(&metadata))
    }
}
