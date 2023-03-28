/// By deploying this module, the deployer will be creating a new managed fungible asset with the hardcoded
/// maximum supply, name, symbol, and decimals. The address of the asset can be obtained via get_asset().
/// The deployer will also become the initial admin and can mint/burn/freeze/unfreeze accounts.
/// The admin can transfer the asset via object::transfer() at any point to set a new admin.
module fungible_asset::managed_fungible_asset {
    use aptos_framework::fungible_asset::{Self, MintRef, TransferRef, BurnRef, FungibleAsset};
    use aptos_framework::object::{Self, Object};
    use aptos_framework::primary_wallet;
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
    public entry fun mint<T: key>(
        admin: &signer,
        metadata: Object<T>,
        amount: u64,
        to: address
    ) acquires ManagedFungibleAsset {
        let mint_ref = &authorized_borrow_refs<T>(admin, metadata).mint_ref;
        let to_wallet = primary_wallet::ensure_primary_wallet_exists(to, metadata);
        fungible_asset::deposit(to_wallet, fungible_asset::mint(mint_ref, amount));
    }

    /// Transfer as the owner of metadata object ignoring `allow_ungated_transfer` field.
    public entry fun transfer<T: key>(
        admin: &signer,
        metadata: Object<T>,
        from: address,
        to: address,
        amount: u64,
    ) acquires ManagedFungibleAsset {
        let transfer_ref = &authorized_borrow_refs<T>(admin, metadata).transfer_ref;
        let from_wallet = primary_wallet::ensure_primary_wallet_exists(from, metadata);
        let to_wallet = primary_wallet::ensure_primary_wallet_exists(to, metadata);
        fungible_asset::transfer_with_ref(transfer_ref, from_wallet, to_wallet, amount);
    }

    /// Burn fungible assets as the owner of metadata object.
    public entry fun burn<T: key>(
        admin: &signer,
        metadata: Object<T>,
        from: address,
        amount: u64
    ) acquires ManagedFungibleAsset {
        let burn_ref = &authorized_borrow_refs<T>(admin, metadata).burn_ref;
        let from_wallet = primary_wallet::ensure_primary_wallet_exists(from, metadata);
        fungible_asset::burn(burn_ref, from_wallet, amount);
    }

    /// Freeze an account so it cannot transfer or receive fungible assets.
    public entry fun freeze_account<T: key>(
        admin: &signer,
        metadata: Object<T>,
        account: address,
    ) acquires ManagedFungibleAsset {
        let transfer_ref = &authorized_borrow_refs<T>(admin, metadata).transfer_ref;
        let wallet = primary_wallet::ensure_primary_wallet_exists(account, metadata);
        fungible_asset::set_ungated_transfer(transfer_ref, wallet, false);
    }

    /// Unfreeze an account so it can transfer or receive fungible assets.
    public entry fun unfreeze_account<T: key>(
        admin: &signer,
        metadata: Object<T>,
        account: address,
    ) acquires ManagedFungibleAsset {
        let transfer_ref = &authorized_borrow_refs<T>(admin, metadata).transfer_ref;
        let wallet = primary_wallet::ensure_primary_wallet_exists(account, metadata);
        fungible_asset::set_ungated_transfer(transfer_ref, wallet, true);
    }

    /// Withdraw as the owner of metadata object ignoring `allow_ungated_transfer` field.
    public fun withdraw<T: key>(
        admin: &signer,
        metadata: Object<T>,
        amount: u64,
        from: address,
    ): FungibleAsset acquires ManagedFungibleAsset {
        let transfer_ref = &authorized_borrow_refs<T>(admin, metadata).transfer_ref;
        let from_wallet = primary_wallet::ensure_primary_wallet_exists(from, metadata);
        fungible_asset::withdraw_with_ref(transfer_ref, from_wallet, amount)
    }

    /// Deposit as the owner of metadata object ignoring `allow_ungated_transfer` field.
    public fun deposit<T: key>(
        admin: &signer,
        metadata: Object<T>,
        to: address,
        fa: FungibleAsset
    ) acquires ManagedFungibleAsset {
        let transfer_ref = &authorized_borrow_refs<T>(admin, metadata).transfer_ref;
        let to_wallet = primary_wallet::ensure_primary_wallet_exists(to, metadata);
        fungible_asset::deposit_with_ref(transfer_ref, to_wallet, fa);
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
    fun get_metadata(creator: &signer): Object<FungibleAssetMetadata> {
        let addr = object::create_object_address(&signer::address_of(creator), ASSET_SYMBOL);
        object::address_to_object<FungibleAssetMetadata>(addr)
    }

    #[test(creator = @0xcafe)]
    fun test_basic_flow(
        creator: &signer,
    ) acquires ManagedFungibleAsset {
        init_module(creator);
        let metadata = get_metadata(creator);
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
        init_module(creator);
        let metadata = get_metadata(creator);
        let creator_address = signer::address_of(creator);
        mint(aaron, metadata, 100, creator_address);
    }
}
