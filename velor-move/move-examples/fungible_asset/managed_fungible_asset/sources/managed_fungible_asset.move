/// This module provides a managed fungible asset that allows the owner of the metadata object to
/// mint, transfer and burn fungible assets.
///
/// The functionalities offered by this module are:
/// 1. Mint fungible assets to fungible stores as the owner of metadata object.
/// 2. Transfer fungible assets as the owner of metadata object ignoring `frozen` field between fungible stores.
/// 3. Burn fungible assets from fungible stores as the owner of metadata object.
/// 4. Withdraw the merged fungible assets from fungible stores as the owner of metadata object.
/// 5. Deposit fungible assets to fungible stores.
module example_addr::managed_fungible_asset {
    use velor_framework::fungible_asset::{Self, MintRef, TransferRef, BurnRef, Metadata, FungibleStore, FungibleAsset};
    use velor_framework::object::{Self, Object, ConstructorRef};
    use velor_framework::primary_fungible_store;
    use std::error;
    use std::signer;
    use std::string::String;
    use std::option;

    /// Only fungible asset metadata owner can make changes.
    const ERR_NOT_OWNER: u64 = 1;
    /// The length of ref_flags is not 3.
    const ERR_INVALID_REF_FLAGS_LENGTH: u64 = 2;
    /// The lengths of two vector do not equal.
    const ERR_VECTORS_LENGTH_MISMATCH: u64 = 3;
    /// MintRef error.
    const ERR_MINT_REF: u64 = 4;
    /// TransferRef error.
    const ERR_TRANSFER_REF: u64 = 5;
    /// BurnRef error.
    const ERR_BURN_REF: u64 = 6;

    #[resource_group_member(group = velor_framework::object::ObjectGroup)]
    /// Hold refs to control the minting, transfer and burning of fungible assets.
    struct ManagingRefs has key {
        mint_ref: Option<MintRef>,
        transfer_ref: Option<TransferRef>,
        burn_ref: Option<BurnRef>,
    }

    /// Initialize metadata object and store the refs specified by `ref_flags`.
    public fun initialize(
        constructor_ref: &ConstructorRef,
        maximum_supply: u128,
        name: String,
        symbol: String,
        decimals: u8,
        icon_uri: String,
        project_uri: String,
        ref_flags: vector<bool>,
    ) {
        assert!(vector::length(&ref_flags) == 3, error::invalid_argument(ERR_INVALID_REF_FLAGS_LENGTH));
        let supply = if (maximum_supply != 0) {
            option::some(maximum_supply)
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
            project_uri,
        );

        // Optionally create mint/burn/transfer refs to allow creator to manage the fungible asset.
        let mint_ref = if (*vector::borrow(&ref_flags, 0)) {
            option::some(fungible_asset::generate_mint_ref(constructor_ref))
        } else {
            option::none()
        };
        let transfer_ref = if (*vector::borrow(&ref_flags, 1)) {
            option::some(fungible_asset::generate_transfer_ref(constructor_ref))
        } else {
            option::none()
        };
        let burn_ref = if (*vector::borrow(&ref_flags, 2)) {
            option::some(fungible_asset::generate_burn_ref(constructor_ref))
        } else {
            option::none()
        };
        let metadata_object_signer = object::generate_signer(constructor_ref);
        move_to(
            &metadata_object_signer,
            ManagingRefs { mint_ref, transfer_ref, burn_ref }
        )
    }

    /// Mint as the owner of metadata object to the primary fungible stores of the accounts with amounts of FAs.
    public entry fun mint_to_primary_stores(
        admin: &signer,
        asset: Object<Metadata>,
        to: vector<address>,
        amounts: vector<u64>
    ) acquires ManagingRefs {
        let receiver_primary_stores = vector::map(
            to,
            |addr| primary_fungible_store::ensure_primary_store_exists(addr, asset)
        );
        mint(admin, asset, receiver_primary_stores, amounts);
    }


    /// Mint as the owner of metadata object to multiple fungible stores with amounts of FAs.
    public entry fun mint(
        admin: &signer,
        asset: Object<Metadata>,
        stores: vector<Object<FungibleStore>>,
        amounts: vector<u64>,
    ) acquires ManagingRefs {
        let length = vector::length(&stores);
        assert!(length == vector::length(&amounts), error::invalid_argument(ERR_VECTORS_LENGTH_MISMATCH));
        let mint_ref = authorized_borrow_mint_ref(admin, asset);
        let i = 0;
        while (i < length) {
            fungible_asset::mint_to(mint_ref, *vector::borrow(&stores, i), *vector::borrow(&amounts, i));
            i = i + 1;
        }
    }

    /// Transfer as the owner of metadata object ignoring `frozen` field from primary stores to primary stores of
    /// accounts.
    public entry fun transfer_between_primary_stores(
        admin: &signer,
        asset: Object<Metadata>,
        from: vector<address>,
        to: vector<address>,
        amounts: vector<u64>
    ) acquires ManagingRefs {
        let sender_primary_stores = vector::map(
            from,
            |addr| primary_fungible_store::primary_store(addr, asset)
        );
        let receiver_primary_stores = vector::map(
            to,
            |addr| primary_fungible_store::ensure_primary_store_exists(addr, asset)
        );
        transfer(admin, asset, sender_primary_stores, receiver_primary_stores, amounts);
    }

    /// Transfer as the owner of metadata object ignoring `frozen` field between fungible stores.
    public entry fun transfer(
        admin: &signer,
        asset: Object<Metadata>,
        sender_stores: vector<Object<FungibleStore>>,
        receiver_stores: vector<Object<FungibleStore>>,
        amounts: vector<u64>,
    ) acquires ManagingRefs {
        let length = vector::length(&sender_stores);
        assert!(length == vector::length(&receiver_stores), error::invalid_argument(ERR_VECTORS_LENGTH_MISMATCH));
        assert!(length == vector::length(&amounts), error::invalid_argument(ERR_VECTORS_LENGTH_MISMATCH));
        let transfer_ref = authorized_borrow_transfer_ref(admin, asset);
        let i = 0;
        while (i < length) {
            fungible_asset::transfer_with_ref(
                transfer_ref,
                *vector::borrow(&sender_stores, i),
                *vector::borrow(&receiver_stores, i),
                *vector::borrow(&amounts, i)
            );
            i = i + 1;
        }
    }

    /// Burn fungible assets as the owner of metadata object from the primary stores of accounts.
    public entry fun burn_from_primary_stores(
        admin: &signer,
        asset: Object<Metadata>,
        from: vector<address>,
        amounts: vector<u64>
    ) acquires ManagingRefs {
        let primary_stores = vector::map(
            from,
            |addr| primary_fungible_store::primary_store(addr, asset)
        );
        burn(admin, asset, primary_stores, amounts);
    }

    /// Burn fungible assets as the owner of metadata object from fungible stores.
    public entry fun burn(
        admin: &signer,
        asset: Object<Metadata>,
        stores: vector<Object<FungibleStore>>,
        amounts: vector<u64>
    ) acquires ManagingRefs {
        let length = vector::length(&stores);
        assert!(length == vector::length(&amounts), error::invalid_argument(ERR_VECTORS_LENGTH_MISMATCH));
        let burn_ref = authorized_borrow_burn_ref(admin, asset);
        let i = 0;
        while (i < length) {
            fungible_asset::burn_from(burn_ref, *vector::borrow(&stores, i), *vector::borrow(&amounts, i));
            i = i + 1;
        };
    }


    /// Freeze/unfreeze the primary stores of accounts so they cannot transfer or receive fungible assets.
    public entry fun set_primary_stores_frozen_status(
        admin: &signer,
        asset: Object<Metadata>,
        accounts: vector<address>,
        frozen: bool
    ) acquires ManagingRefs {
        let primary_stores = vector::map(accounts, |acct| {
            primary_fungible_store::ensure_primary_store_exists(acct, asset)
        });
        set_frozen_status(admin, asset, primary_stores, frozen);
    }

    /// Freeze/unfreeze the fungible stores so they cannot transfer or receive fungible assets.
    public entry fun set_frozen_status(
        admin: &signer,
        asset: Object<Metadata>,
        stores: vector<Object<FungibleStore>>,
        frozen: bool
    ) acquires ManagingRefs {
        let transfer_ref = authorized_borrow_transfer_ref(admin, asset);
        vector::for_each(stores, |store| {
            fungible_asset::set_frozen_flag(transfer_ref, store, frozen);
        });
    }

    /// Withdraw as the owner of metadata object ignoring `frozen` field from primary fungible stores of accounts.
    public fun withdraw_from_primary_stores(
        admin: &signer,
        asset: Object<Metadata>,
        from: vector<address>,
        amounts: vector<u64>
    ): FungibleAsset acquires ManagingRefs {
        let primary_stores = vector::map(
            from,
            |addr| primary_fungible_store::primary_store(addr, asset)
        );
        withdraw(admin, asset, primary_stores, amounts)
    }

    /// Withdraw as the owner of metadata object ignoring `frozen` field from fungible stores.
    /// return a fungible asset `fa` where `fa.amount = sum(amounts)`.
    public fun withdraw(
        admin: &signer,
        asset: Object<Metadata>,
        stores: vector<Object<FungibleStore>>,
        amounts: vector<u64>
    ): FungibleAsset acquires ManagingRefs {
        let length = vector::length(&stores);
        assert!(length == vector::length(&amounts), error::invalid_argument(ERR_VECTORS_LENGTH_MISMATCH));
        let transfer_ref = authorized_borrow_transfer_ref(admin, asset);
        let i = 0;
        let sum = fungible_asset::zero(asset);
        while (i < length) {
            let fa = fungible_asset::withdraw_with_ref(
                transfer_ref,
                *vector::borrow(&stores, i),
                *vector::borrow(&amounts, i)
            );
            fungible_asset::merge(&mut sum, fa);
            i = i + 1;
        };
        sum
    }

    /// Deposit as the owner of metadata object ignoring `frozen` field to primary fungible stores of accounts from a
    /// single source of fungible asset.
    public fun deposit_to_primary_stores(
        admin: &signer,
        fa: &mut FungibleAsset,
        from: vector<address>,
        amounts: vector<u64>,
    ) acquires ManagingRefs {
        let primary_stores = vector::map(
            from,
            |addr| primary_fungible_store::ensure_primary_store_exists(addr, fungible_asset::asset_metadata(fa))
        );
        deposit(admin, fa, primary_stores, amounts);
    }

    /// Deposit as the owner of metadata object ignoring `frozen` field from fungible stores. The amount left in `fa`
    /// is `fa.amount - sum(amounts)`.
    public fun deposit(
        admin: &signer,
        fa: &mut FungibleAsset,
        stores: vector<Object<FungibleStore>>,
        amounts: vector<u64>
    ) acquires ManagingRefs {
        let length = vector::length(&stores);
        assert!(length == vector::length(&amounts), error::invalid_argument(ERR_VECTORS_LENGTH_MISMATCH));
        let transfer_ref = authorized_borrow_transfer_ref(admin, fungible_asset::asset_metadata(fa));
        let i = 0;
        while (i < length) {
            let split_fa = fungible_asset::extract(fa, *vector::borrow(&amounts, i));
            fungible_asset::deposit_with_ref(
                transfer_ref,
                *vector::borrow(&stores, i),
                split_fa,
            );
            i = i + 1;
        };
    }

    /// Borrow the immutable reference of the refs of `metadata`.
    /// This validates that the signer is the metadata object's owner.
    inline fun authorized_borrow_refs(
        owner: &signer,
        asset: Object<Metadata>,
    ): &ManagingRefs acquires ManagingRefs {
        assert!(object::is_owner(asset, signer::address_of(owner)), error::permission_denied(ERR_NOT_OWNER));
        borrow_global<ManagingRefs>(object::object_address(&asset))
    }

    /// Check the existence and borrow `MintRef`.
    inline fun authorized_borrow_mint_ref(
        owner: &signer,
        asset: Object<Metadata>,
    ): &MintRef acquires ManagingRefs {
        let refs = authorized_borrow_refs(owner, asset);
        assert!(option::is_some(&refs.mint_ref), error::not_found(ERR_MINT_REF));
        option::borrow(&refs.mint_ref)
    }

    /// Check the existence and borrow `TransferRef`.
    inline fun authorized_borrow_transfer_ref(
        owner: &signer,
        asset: Object<Metadata>,
    ): &TransferRef acquires ManagingRefs {
        let refs = authorized_borrow_refs(owner, asset);
        assert!(option::is_some(&refs.transfer_ref), error::not_found(ERR_TRANSFER_REF));
        option::borrow(&refs.transfer_ref)
    }

    /// Check the existence and borrow `BurnRef`.
    inline fun authorized_borrow_burn_ref(
        owner: &signer,
        asset: Object<Metadata>,
    ): &BurnRef acquires ManagingRefs {
        let refs = authorized_borrow_refs(owner, asset);
        assert!(option::is_some(&refs.mint_ref), error::not_found(ERR_BURN_REF));
        option::borrow(&refs.burn_ref)
    }

    #[test_only]
    use velor_framework::object::object_from_constructor_ref;
    #[test_only]
    use std::string::utf8;
    use std::vector;
    use std::option::Option;

    #[test_only]
    fun create_test_mfa(creator: &signer): Object<Metadata> {
        let constructor_ref = &object::create_named_object(creator, b"APT");
        initialize(
            constructor_ref,
            0,
            utf8(b"Velor Token"), /* name */
            utf8(b"APT"), /* symbol */
            8, /* decimals */
            utf8(b"http://example.com/favicon.ico"), /* icon */
            utf8(b"http://example.com"), /* project */
            vector[true, true, true]
        );
        object_from_constructor_ref<Metadata>(constructor_ref)
    }

    #[test(creator = @example_addr)]
    fun test_basic_flow(
        creator: &signer,
    ) acquires ManagingRefs {
        let metadata = create_test_mfa(creator);
        let creator_address = signer::address_of(creator);
        let aaron_address = @0xface;

        mint_to_primary_stores(creator, metadata, vector[creator_address, aaron_address], vector[100, 50]);
        assert!(primary_fungible_store::balance(creator_address, metadata) == 100, 1);
        assert!(primary_fungible_store::balance(aaron_address, metadata) == 50, 2);

        set_primary_stores_frozen_status(creator, metadata, vector[creator_address, aaron_address], true);
        assert!(primary_fungible_store::is_frozen(creator_address, metadata), 3);
        assert!(primary_fungible_store::is_frozen(aaron_address, metadata), 4);

        transfer_between_primary_stores(
            creator,
            metadata,
            vector[creator_address, aaron_address],
            vector[aaron_address, creator_address],
            vector[10, 5]
        );
        assert!(primary_fungible_store::balance(creator_address, metadata) == 95, 5);
        assert!(primary_fungible_store::balance(aaron_address, metadata) == 55, 6);

        set_primary_stores_frozen_status(creator, metadata, vector[creator_address, aaron_address], false);
        assert!(!primary_fungible_store::is_frozen(creator_address, metadata), 7);
        assert!(!primary_fungible_store::is_frozen(aaron_address, metadata), 8);

        let fa = withdraw_from_primary_stores(
            creator,
            metadata,
            vector[creator_address, aaron_address],
            vector[25, 15]
        );
        assert!(fungible_asset::amount(&fa) == 40, 9);
        deposit_to_primary_stores(creator, &mut fa, vector[creator_address, aaron_address], vector[30, 10]);
        fungible_asset::destroy_zero(fa);

        burn_from_primary_stores(creator, metadata, vector[creator_address, aaron_address], vector[100, 50]);
        assert!(primary_fungible_store::balance(creator_address, metadata) == 0, 10);
        assert!(primary_fungible_store::balance(aaron_address, metadata) == 0, 11);
    }

    #[test(creator = @example_addr, aaron = @0xface)]
    #[expected_failure(abort_code = 0x50001, location = Self)]
    fun test_permission_denied(
        creator: &signer,
        aaron: &signer
    ) acquires ManagingRefs {
        let metadata = create_test_mfa(creator);
        let creator_address = signer::address_of(creator);
        mint_to_primary_stores(aaron, metadata, vector[creator_address], vector[100]);
    }
}
