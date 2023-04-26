/// By deploying this module, the deployer provide an extension layer upon fungible asset that helps manage
/// the refs for the deployer, who is set to be the initial admin that can mint/burn/freeze/unfreeze accounts.
/// The admin can transfer the asset via object::transfer() at any point to set a new admin.
module fungible_asset_extension::managed_fungible_asset {
    use aptos_framework::fungible_asset::{Self, MintRef, TransferRef, BurnRef, FungibleAsset, Metadata};
    use aptos_framework::object::{Self, Object, ConstructorRef};
    use aptos_framework::primary_fungible_store;
    use std::error;
    use std::signer;
    use std::string::String;
    use std::option;

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
    public fun initialize(
        constructor_ref: &ConstructorRef,
        monitoring_supply: bool,
        maximum_supply: u128,
        name: String,
        symbol: String,
        decimals: u8,
        icon_uri: String,
        issuer: String,
    ) {
        let supply = if (monitoring_supply) {
            option::some(if (maximum_supply != 0) {
                option::some(maximum_supply)
            } else {
                option::none()
            })
        } else {
            option::none()
        };
        primary_fungible_store::create_primary_store_enabled_fungible_asset(
            constructor_ref,
            supply,
            name,
            symbol,
            decimals,
            icon_uri,
            issuer,
        );

        // Create mint/burn/transfer refs to allow creator to manage the fungible asset.
        let mint_ref = fungible_asset::generate_mint_ref(constructor_ref);
        let burn_ref = fungible_asset::generate_burn_ref(constructor_ref);
        let transfer_ref = fungible_asset::generate_transfer_ref(constructor_ref);
        let metadata_object_signer = object::generate_signer(constructor_ref);
        move_to(
            &metadata_object_signer,
            ManagedFungibleAsset { mint_ref, transfer_ref, burn_ref }
        )
    }

    /// Mint as the owner of metadata object.
    public entry fun mint(
        admin: &signer,
        metadata: Object<Metadata>,
        amount: u64,
        to: address
    ) acquires ManagedFungibleAsset {
        let managed_fungible_asset = authorized_borrow_refs(admin, metadata);
        let to_wallet = primary_fungible_store::ensure_primary_store_exists(to, metadata);
        let fa = fungible_asset::mint(&managed_fungible_asset.mint_ref, amount);
        fungible_asset::deposit_with_ref(&managed_fungible_asset.transfer_ref, to_wallet, fa);
    }

    /// Transfer as the owner of metadata object ignoring `frozen` field.
    public entry fun transfer(
        admin: &signer,
        metadata: Object<Metadata>,
        from: address,
        to: address,
        amount: u64
    ) acquires ManagedFungibleAsset {
        let transfer_ref = &authorized_borrow_refs(admin, metadata).transfer_ref;
        let from_wallet = primary_fungible_store::ensure_primary_store_exists(from, metadata);
        let to_wallet = primary_fungible_store::ensure_primary_store_exists(to, metadata);
        fungible_asset::transfer_with_ref(transfer_ref, from_wallet, to_wallet, amount);
    }

    /// Burn fungible assets as the owner of metadata object.
    public entry fun burn(
        admin: &signer,
        metadata: Object<Metadata>,
        from: address,
        amount: u64
    ) acquires ManagedFungibleAsset {
        let burn_ref = &authorized_borrow_refs(admin, metadata).burn_ref;
        let from_wallet = primary_fungible_store::ensure_primary_store_exists(from, metadata);
        fungible_asset::burn_from(burn_ref, from_wallet, amount);
    }

    /// Freeze an account so it cannot transfer or receive fungible assets.
    public entry fun freeze_account(
        admin: &signer,
        metadata: Object<Metadata>,
        account: address
    ) acquires ManagedFungibleAsset {
        let transfer_ref = &authorized_borrow_refs(admin, metadata).transfer_ref;
        let wallet = primary_fungible_store::ensure_primary_store_exists(account, metadata);
        fungible_asset::set_frozen_flag(transfer_ref, wallet, true);
    }

    /// Unfreeze an account so it can transfer or receive fungible assets.
    public entry fun unfreeze_account(admin: &signer,
                                      metadata: Object<Metadata>,
                                      account: address) acquires ManagedFungibleAsset {
        let transfer_ref = &authorized_borrow_refs(admin, metadata).transfer_ref;
        let wallet = primary_fungible_store::ensure_primary_store_exists(account, metadata);
        fungible_asset::set_frozen_flag(transfer_ref, wallet, false);
    }

    /// Withdraw as the owner of metadata object ignoring `frozen` field.
    public fun withdraw(admin: &signer,
                        metadata: Object<Metadata>,
                        amount: u64, from: address): FungibleAsset acquires ManagedFungibleAsset {
        let transfer_ref = &authorized_borrow_refs(admin, metadata).transfer_ref;
        let from_wallet = primary_fungible_store::ensure_primary_store_exists(from, metadata);
        fungible_asset::withdraw_with_ref(transfer_ref, from_wallet, amount)
    }

    /// Deposit as the owner of metadata object ignoring `frozen` field.
    public fun deposit(admin: &signer,
                       metadata: Object<Metadata>,
                       to: address, fa: FungibleAsset) acquires ManagedFungibleAsset {
        let transfer_ref = &authorized_borrow_refs(admin, metadata).transfer_ref;
        let to_wallet = primary_fungible_store::ensure_primary_store_exists(to, metadata);
        fungible_asset::deposit_with_ref(transfer_ref, to_wallet, fa);
    }

    /// Borrow the immutable reference of the refs of `metadata`.
    /// This validates that the signer is the metadata object's owner.
    inline fun authorized_borrow_refs(
        owner: &signer,
        asset: Object<Metadata>,
    ): &ManagedFungibleAsset acquires ManagedFungibleAsset {
        assert!(object::is_owner(asset, signer::address_of(owner)), error::permission_denied(ENOT_OWNER));
        borrow_global<ManagedFungibleAsset>(object::object_address(&asset))
    }

    #[test_only]
    use aptos_framework::object::object_from_constructor_ref;
    #[test_only]
    use std::string::utf8;

    #[test_only]
    fun create_test_mfa(creator: &signer): Object<Metadata> {
        let constructor_ref = &object::create_named_object(creator, b"APT");
        initialize(
            constructor_ref,
            true,
            0,
            utf8(b"Aptos Token"), /* name */
            utf8(b"APT"), /* symbol */
            8, /* decimals */
            utf8(b"http://example.com"), /* icon */
            utf8(b"My issuer"), /* issuer */
        );
        object_from_constructor_ref<Metadata>(constructor_ref)
    }

    #[test(creator = @0xcafe)]
    fun test_basic_flow(
        creator: &signer,
    ) acquires ManagedFungibleAsset {
        let metadata = create_test_mfa(creator);
        let creator_address = signer::address_of(creator);
        let aaron_address = @0xface;

        mint(creator, metadata, 100, creator_address);
        assert!(primary_fungible_store::balance(creator_address, metadata) == 100, 4);
        freeze_account(creator, metadata, creator_address);
        assert!(primary_fungible_store::is_frozen(creator_address, metadata), 5);
        transfer(creator, metadata, creator_address, aaron_address, 10);
        assert!(primary_fungible_store::balance(aaron_address, metadata) == 10, 6);

        unfreeze_account(creator, metadata, creator_address);
        assert!(!primary_fungible_store::is_frozen(creator_address, metadata), 7);
        burn(creator, metadata, creator_address, 90);
    }

    #[test(creator = @0xcafe, aaron = @0xface)]
    #[expected_failure(abort_code = 0x50001, location = Self)]
    fun test_permission_denied(
        creator: &signer,
        aaron: &signer
    ) acquires ManagedFungibleAsset {
        let metadata = create_test_mfa(creator);
        let creator_address = signer::address_of(creator);
        mint(aaron, metadata, 100, creator_address);
    }
}
