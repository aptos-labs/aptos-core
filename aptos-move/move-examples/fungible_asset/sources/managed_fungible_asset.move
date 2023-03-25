/// This module provides an addtional ready-to-use solution on top of `FungibleAssetMetadata` that manages the refs of
/// mint, burn and transfer for the creator in a straightforward scheme. It offers creators to destory any refs in an
/// on-demand manner too.
module fungible_asset::managed_fungible_asset {
    use aptos_framework::fungible_asset::{Self, MintRef, TransferRef, BurnRef, FungibleAsset};
    use aptos_framework::object::{Self, Object, ConstructorRef};
    use aptos_framework::primary_wallet;
    use std::error;
    use std::signer;
    use std::string::String;

    /// Only fungible asset metadata owner can make changes.
    const ENOT_OWNER: u64 = 1;

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Hold refs to control the minting, transfer and burning of fungible assets.
    struct ManagedFungibleAsset has key {
        mint_ref: MintRef,
        transfer_ref: TransferRef,
        burn_ref: BurnRef,
    }

    /// Initialize metadata object and store the refs.
    public fun init_managed_fungible_asset(
        constructor_ref: &ConstructorRef,
        maximum_supply: u64,
        name: String,
        symbol: String,
        decimals: u8
    ) {
        let (mint_ref, transfer_ref, burn_ref) = fungible_asset::init_metadata(
            constructor_ref,
            maximum_supply,
            name,
            symbol,
            decimals
        );
        let metadata_object_signer = object::generate_signer(constructor_ref);
        move_to(
            &metadata_object_signer,
            ManagedFungibleAsset { mint_ref, transfer_ref, burn_ref }
        )
    }

    /// Mint as the owner of metadata object.
    public fun mint<T: key>(
        metadata_owner: &signer,
        metadata: Object<T>,
        amount: u64,
        to: address
    ) acquires ManagedFungibleAsset {
        let mint_ref = &authorized_borrow_refs<T>(metadata_owner, metadata).mint_ref;
        let to_wallet = primary_wallet::ensure_primary_wallet_exists(to, metadata);
        fungible_asset::deposit(to_wallet, fungible_asset::mint(mint_ref, amount));
    }

    /// Withdraw as the owner of metadata object ignoring `allow_ungated_transfer` field.
    public fun withdraw<T: key>(
        metadata_owner: &signer,
        metadata: Object<T>,
        amount: u64,
        from: address,
    ): FungibleAsset acquires ManagedFungibleAsset {
        let transfer_ref = &authorized_borrow_refs<T>(metadata_owner, metadata).transfer_ref;
        let from_wallet = primary_wallet::ensure_primary_wallet_exists(from, metadata);
        fungible_asset::withdraw_with_ref(transfer_ref, from_wallet, amount)
    }

    /// Deposit as the owner of metadata object ignoring `allow_ungated_transfer` field.
    public fun deposit<T: key>(
        metadata_owner: &signer,
        metadata: Object<T>,
        to: address,
        fa: FungibleAsset
    ) acquires ManagedFungibleAsset {
        let transfer_ref = &authorized_borrow_refs<T>(metadata_owner, metadata).transfer_ref;
        let to_wallet = primary_wallet::ensure_primary_wallet_exists(to, metadata);
        fungible_asset::deposit_with_ref(transfer_ref, to_wallet, fa);
    }

    /// Transfer as the owner of metadata object ignoring `allow_ungated_transfer` field.
    public fun transfer<T: key>(
        metadata_owner: &signer,
        metadata: Object<T>,
        from: address,
        to: address,
        amount: u64,
    ) acquires ManagedFungibleAsset {
        let transfer_ref = &authorized_borrow_refs<T>(metadata_owner, metadata).transfer_ref;
        let from_wallet = primary_wallet::ensure_primary_wallet_exists(from, metadata);
        let to_wallet = primary_wallet::ensure_primary_wallet_exists(to, metadata);
        fungible_asset::transfer_with_ref(transfer_ref, from_wallet, amount, to_wallet);
    }

    /// Burn fungible assets as the owner of metadata object.
    public fun burn<T: key>(
        metadata_owner: &signer,
        metadata: Object<T>,
        from: address,
        amount: u64
    ) acquires ManagedFungibleAsset {
        let burn_ref = &authorized_borrow_refs<T>(metadata_owner, metadata).burn_ref;
        let from_wallet = primary_wallet::ensure_primary_wallet_exists(from, metadata);
        fungible_asset::burn(burn_ref, from_wallet, amount);
    }

    /// Freeze an account so it cannot transfer or receive fungible assets.
    public fun freeze_account<T: key>(
        metadata_owner: &signer,
        metadata: Object<T>,
        account: address,
    ) acquires ManagedFungibleAsset {
        let transfer_ref = &authorized_borrow_refs<T>(metadata_owner, metadata).transfer_ref;
        let wallet = primary_wallet::ensure_primary_wallet_exists(account, metadata);
        fungible_asset::set_ungated_transfer(transfer_ref, wallet, false);
    }

    /// Unfreeze an account so it can transfer or receive fungible assets.
    public fun unfreeze_account<T: key>(
        metadata_owner: &signer,
        metadata: Object<T>,
        account: address,
    ) acquires ManagedFungibleAsset {
        let transfer_ref = &authorized_borrow_refs<T>(metadata_owner, metadata).transfer_ref;
        let wallet = primary_wallet::ensure_primary_wallet_exists(account, metadata);
        fungible_asset::set_ungated_transfer(transfer_ref, wallet, true);
    }

    /// Borrow the immutable reference of the refs of `metadata`.
    /// This validates that the signer is the metadata object's owner.
    inline fun authorized_borrow_refs<T: key>(
        owner: &signer,
        metadata: Object<T>,
    ): &ManagedFungibleAsset acquires ManagedFungibleAsset {
        assert!(object::is_owner(metadata, signer::address_of(owner)), error::permission_denied(ENOT_OWNER));
        borrow_global<ManagedFungibleAsset>(object::object_address(&metadata))
    }

    #[test_only]
    use aptos_framework::fungible_asset::{FungibleAssetMetadata};
    #[test_only]
    use std::string;

    #[test_only]
    public fun init_test_managing_refs(creator: &signer): Object<FungibleAssetMetadata> {
        let (constructor_ref, _) = fungible_asset::create_test_token(creator);
        init_managed_fungible_asset(
            &constructor_ref,
            100 /* max supply */,
            string::utf8(b"USDA"),
            string::utf8(b"$$$"),
            0
        );
        object::object_from_constructor_ref<FungibleAssetMetadata>(&constructor_ref)
    }

    #[test(creator = @0xcafe)]
    fun test_basic_flow(
        creator: &signer,
    ) acquires ManagedFungibleAsset {
        let metadata = init_test_managing_refs(creator);
        let creator_address = signer::address_of(creator);
        let aaron_address = @0xface;

        mint(creator, metadata, 100, creator_address);
        assert!(primary_wallet::balance(creator_address, metadata) == 100, 4);
        freeze_account(creator, metadata, creator_address);
        assert!(!primary_wallet::ungated_transfer_allowed(creator_address, metadata), 5);
        transfer(creator, metadata, creator_address, aaron_address, 10);
        assert!(primary_wallet::balance(aaron_address, metadata) == 10, 6);

        unfreeze_account(creator, metadata, creator_address);
        assert!(primary_wallet::ungated_transfer_allowed(creator_address, metadata), 7);
        burn(creator, metadata, creator_address, 90);
    }

    #[test(creator = @0xcafe, aaron = @0xface)]
    #[expected_failure(abort_code = 0x50001, location = Self)]
    fun test_permission_denied(
        creator: &signer,
        aaron: &signer
    ) acquires ManagedFungibleAsset {
        let metadata = init_test_managing_refs(creator);
        let creator_address = signer::address_of(creator);
        mint(aaron, metadata, 100, creator_address);
    }
}
