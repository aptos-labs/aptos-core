/// This module provides an addtional ready-to-use solution on top of `FungibleAssetMetadata` that manages the refs of
/// mint, burn and transfer for the creator in a straightforward scheme. It offers creators to destory any refs in an
/// on-demand manner too.
module aptos_framework::managed_fungible_metadata {
    use aptos_framework::fungible_asset::{Self, MintRef, TransferRef, BurnRef, FungibleAsset};
    use aptos_framework::fungible_store;
    use aptos_framework::object::{Self, Object, ConstructorRef};
    use std::error;
    use std::option::{Self, Option};
    use std::signer;
    use std::string::String;

    /// MintRef existence error.
    const EMINT_REF: u64 = 1;
    /// TransferRef existence error.
    const ETRANSFER_REF: u64 = 2;
    /// BurnRef existence error.
    const EBURN_REF: u64 = 3;
    /// Not the owner.
    const ENOT_OWNER: u64 = 4;
    /// Refs existence errors.
    const EMANAGED_FUNGIBLE_ASSET_REFS: u64 = 5;

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Hold refs to control the minting, transfer and burning of fungible assets.
    struct ManagingRefs has key {
        mint: Option<MintRef>,
        transfer: Option<TransferRef>,
        burn: Option<BurnRef>,
    }

    /// Initialize metadata object and store the refs.
    public fun init_managing_refs(
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
            ManagingRefs {
                mint: option::some(mint_ref), transfer: option::some(transfer_ref), burn: option::some(burn_ref)
            }
        )
    }

    /// Mint as the owner of metadata object.
    public fun mint<T: key>(
        metadata_owner: &signer,
        metadata: &Object<T>,
        amount: u64,
        to: address
    ) acquires ManagingRefs {
        assert_owner(metadata_owner, metadata);
        let mint_ref = borrow_mint_from_refs(metadata);
        let fa = fungible_asset::mint(mint_ref, amount);
        fungible_store::deposit(fa, to);
    }

    /// Withdraw as the owner of metadata object ignoring `allow_ungated_transfer` field.
    public fun withdraw<T: key>(
        metadata_owner: &signer,
        metadata: &Object<T>,
        amount: u64,
        from: address,
    ): FungibleAsset acquires ManagingRefs {
        assert_owner(metadata_owner, metadata);
        let transfer_ref = borrow_transfer_from_refs(metadata);
        fungible_store::withdraw_with_ref(transfer_ref, from, amount)
    }

    /// Deposit as the owner of metadata object ignoring `allow_ungated_transfer` field.
    public fun deposit<T: key>(
        metadata_owner: &signer,
        metadata: &Object<T>,
        to: address,
        fa: FungibleAsset
    ) acquires ManagingRefs {
        assert_owner(metadata_owner, metadata);
        let transfer_ref = borrow_transfer_from_refs(metadata);
        fungible_store::deposit_with_ref(transfer_ref, to, fa);
    }

    /// Transfer as the owner of metadata object ignoring `allow_ungated_transfer` field.
    public fun transfer<T: key>(
        metadata_owner: &signer,
        metadata: &Object<T>,
        from: address,
        to: address,
        amount: u64,
    ) acquires ManagingRefs {
        assert_owner(metadata_owner, metadata);
        let transfer_ref = borrow_transfer_from_refs(metadata);
        fungible_store::transfer_with_ref(transfer_ref, from, to, amount);
    }

    /// Burn fungible assets as the owner of metadata object.
    public fun burn<T: key>(
        metadata_owner: &signer,
        metadata: &Object<T>,
        from: address,
        amount: u64
    ) acquires ManagingRefs {
        assert_owner(metadata_owner, metadata);
        let burn_ref = borrow_burn_from_refs(metadata);
        fungible_store::burn(burn_ref, from, amount);
    }

    /// Set the `allow_ungated_transfer` field in `AccountFungibleAsset` associated with `metadata` of `account` as the
    /// owner of metadata object.
    public fun set_ungated_transfer<T: key>(
        metadata_owner: &signer,
        metadata: &Object<T>,
        account: address,
        allow: bool
    ) acquires ManagingRefs {
        assert_owner(metadata_owner, metadata);
        let transfer_ref = borrow_transfer_from_refs(metadata);
        fungible_store::set_ungated_transfer(transfer_ref, account, allow);
    }

    /// Return if the owner of `metadata` has access to `MintRef`.
    public fun can_mint<T: key>(metadata: &Object<T>): bool acquires ManagingRefs {
        option::is_some(&borrow_refs(metadata).mint)
    }

    /// Return if the owner of `metadata` has access to `TransferRef`.
    public fun can_transfer<T: key>(metadata: &Object<T>): bool acquires ManagingRefs {
        option::is_some(&borrow_refs(metadata).transfer)
    }

    /// Return if the owner of `metadata` has access to `BurnRef`.
    public fun can_burn<T: key>(metadata: &Object<T>): bool acquires ManagingRefs {
        option::is_some(&borrow_refs(metadata).burn)
    }

    /// Let metadata owner to explicitly waive the mint capability.
    public fun waive_mint<T: key>(
        metadata_owner: &signer,
        metadata: &Object<T>
    ) acquires ManagingRefs {
        let mint_ref = &mut borrow_refs_mut(metadata_owner, metadata).mint;
        assert!(option::is_some(mint_ref), error::not_found(EMINT_REF));
        option::extract(mint_ref);
    }

    /// Let metadata owner to explicitly waive the transfer capability.
    public fun waive_transfer<T: key>(
        metadata_owner: &signer,
        metadata: &Object<T>
    ) acquires ManagingRefs {
        let transfer_ref = &mut borrow_refs_mut(metadata_owner, metadata).transfer;
        assert!(option::is_some(transfer_ref), error::not_found(ETRANSFER_REF));
        option::extract(transfer_ref);
    }

    /// Let metadata owner to explicitly waive the burn capability.
    public fun waive_burn<T: key>(
        metadata_owner: &signer,
        metadata: &Object<T>
    ) acquires ManagingRefs {
        let burn_ref = &mut borrow_refs_mut(metadata_owner, metadata).burn;
        assert!(option::is_some(burn_ref), error::not_found(ETRANSFER_REF));
        option::extract(burn_ref);
    }

    /// Borrow the immutable reference of the `MintRef` of `metadata`.
    inline fun borrow_mint_from_refs<T: key>(
        metadata: &Object<T>,
    ): &MintRef acquires ManagingRefs {
        let mint_ref = &borrow_refs(metadata).mint;
        assert!(option::is_some(mint_ref), error::not_found(EMINT_REF));
        option::borrow(mint_ref)
    }

    /// Borrow the immutable reference of the `TransferRef` of `metadata`.
    inline fun borrow_transfer_from_refs<T: key>(
        metadata: &Object<T>,
    ): &TransferRef acquires ManagingRefs {
        let transfer_ref = &borrow_refs(metadata).transfer;
        assert!(option::is_some(transfer_ref), error::not_found(ETRANSFER_REF));
        option::borrow(transfer_ref)
    }

    /// Borrow the immutable reference of the `BurnRef` of `metadata`.
    inline fun borrow_burn_from_refs<T: key>(
        metadata: &Object<T>,
    ): &BurnRef acquires ManagingRefs {
        let burn_ref = &borrow_refs(metadata).burn;
        assert!(option::is_some(burn_ref), error::not_found(EBURN_REF));
        option::borrow(burn_ref)
    }

    /// Borrow the immutable reference of the refs of `metadata`.
    inline fun borrow_refs<T: key>(
        metadata: &Object<T>,
    ): &ManagingRefs acquires ManagingRefs {
        borrow_global_mut<ManagingRefs>(verify(metadata))
    }

    /// Borrow the mutable reference of the refs of `metadata`.
    inline fun borrow_refs_mut<T: key>(
        owner: &signer,
        metadata: &Object<T>,
    ): &mut ManagingRefs acquires ManagingRefs {
        assert_owner(owner, metadata);
        borrow_global_mut<ManagingRefs>(verify(metadata))
    }

    /// Verify `metadata` indeed has `ManagingRefs` resource associated.
    inline fun verify<T: key>(metadata: &Object<T>): address {
        let metadata_addr = object::object_address(metadata);
        object::address_to_object<ManagingRefs>(metadata_addr);
        metadata_addr
    }

    /// Assert the owner of `metadata`.
    inline fun assert_owner<T: key>(owner: &signer, metadata: &Object<T>) {
        assert!(object::is_owner(*metadata, signer::address_of(owner)), error::permission_denied(ENOT_OWNER));
    }

    #[test_only]
    use aptos_framework::fungible_asset::TestToken;
    #[test_only]
    use aptos_framework::fungible_store::{balance, ungated_transfer_allowed};
    #[test_only]
    use std::string;

    #[test_only]
    public fun init_test_managing_refs(creator: &signer): Object<TestToken> {
        let (creator_ref, metadata) = fungible_asset::create_test_token(creator);
        init_managing_refs(
            &creator_ref,
            100 /* max supply */,
            string::utf8(b"USDA"),
            string::utf8(b"$$$"),
            0
        );
        metadata
    }

    #[test(creator = @0xcafe)]
    fun test_basic_flow(
        creator: &signer,
    ) acquires ManagingRefs {
        let metadata = init_test_managing_refs(creator);
        let creator_address = signer::address_of(creator);
        let aaron_address = @0xface;

        assert!(can_mint(&metadata), 1);
        assert!(can_transfer(&metadata), 2);
        assert!(can_burn(&metadata), 3);

        mint(creator, &metadata, 100, creator_address);
        assert!(balance(creator_address, &metadata) == 100, 4);
        set_ungated_transfer(creator, &metadata, creator_address, false);
        assert!(!ungated_transfer_allowed(creator_address, &metadata), 5);
        transfer(creator, &metadata, creator_address, aaron_address, 10);
        assert!(balance(aaron_address, &metadata) == 10, 6);

        set_ungated_transfer(creator, &metadata, creator_address, true);
        assert!(ungated_transfer_allowed(creator_address, &metadata), 7);
        burn(creator, &metadata, creator_address, 90);

        waive_mint(creator, &metadata);
        waive_transfer(creator, &metadata);
        waive_burn(creator, &metadata);

        assert!(!can_mint(&metadata), 8);
        assert!(!can_transfer(&metadata), 9);
        assert!(!can_burn(&metadata), 10);
    }

    #[test(creator = @0xcafe, aaron = @0xface)]
    #[expected_failure(abort_code = 0x50004, location = Self)]
    fun test_permission_denied(
        creator: &signer,
        aaron: &signer
    ) acquires ManagingRefs {
        let metadata = init_test_managing_refs(creator);
        let creator_address = signer::address_of(creator);
        assert!(can_mint(&metadata), 1);
        mint(aaron, &metadata, 100, creator_address);
    }
}
