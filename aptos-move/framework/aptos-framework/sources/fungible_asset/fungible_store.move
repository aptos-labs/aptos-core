module aptos_framework::fungible_store {
    use aptos_framework::create_signer;
    use aptos_framework::fungible_asset::{Self, AccountFungibleAsset, FungibleAsset};
    use aptos_framework::fungible_source::{Self, FungibleSource};
    use aptos_framework::object::{Self, Object};
    use aptos_std::smart_table::{Self, SmartTable};
    use std::option::{Self, Option};
    use std::error;

    friend aptos_framework::fungible_caps;

    /// The account fungible asset object existence error.
    const EACCOUNT_FUNGIBLE_ASSET_OBJECT: u64 = 1;

    /// Represents all the fungible asset objects of an onwer keyed by the address of the base asset object.
    struct FungibleAssetStore has key {
        index: SmartTable<Object<FungibleSource>, Object<AccountFungibleAsset>>
    }

    /// Check the balance of an `AccountFungibleAsset`.
    public fun balance<T: key>(
        account: address,
        asset: &Object<T>
    ): u64 acquires FungibleAssetStore {
        let asset = fungible_source::verify(asset);
        let afa_opt = get_account_fungible_asset_object(
            account,
            &asset,
            false
        );
        if (option::is_none(&afa_opt)) {
            return 0
        };
        let afa = option::destroy_some(afa_opt);
        fungible_asset::balance(&afa)
    }

    /// Check the `AccountFungibleAsset` of `account` allows ungated transfer.
    public fun ungated_transfer_allowed<T: key>(
        account: address,
        asset: &Object<T>
    ): bool acquires FungibleAssetStore {
        let asset = fungible_source::verify(asset);
        let afa_opt = get_account_fungible_asset_object(
            account,
            &asset,
            false
        );
        if (option::is_none(&afa_opt)) {
            return true
        };
        let afa = option::destroy_some(afa_opt);
        fungible_asset::ungated_transfer_allowed(&afa)
    }

    /// Deposit fungible asset to `account`.
    public fun deposit(
        fa: FungibleAsset,
        to: address
    ) acquires FungibleAssetStore {
        let asset = fungible_asset::fungible_asset_source(&fa);
        let afa = ensure_account_fungible_asset_object(
            to,
            &asset,
            true
        );
        fungible_asset::merge(&afa, fa);
    }

    /// Enable/disable the direct transfer of fungible assets.
    public(friend) fun set_ungated_transfer<T: key>(
        account: address,
        asset: &Object<T>,
        allow: bool
    ) acquires FungibleAssetStore {
        let asset = fungible_source::verify(asset);
        let afa_opt = get_account_fungible_asset_object(account, &asset, !allow);
        if (option::is_none(&afa_opt)) {
            return
        };
        let afa = option::destroy_some(afa_opt);
        fungible_asset::set_ungated_transfer(&afa, allow);
        if (fungible_asset::balance(&afa) == 0 && fungible_asset::ungated_transfer_allowed(&afa)) {
            delete_account_fungible_asset_object(account, &asset);
        };
    }

    /// Withdraw `amount` of fungible assets from `account`.
    public(friend) fun withdraw<T: key>(
        account: address,
        asset: &Object<T>,
        amount: u64
    ): FungibleAsset acquires FungibleAssetStore {
        let asset = fungible_source::verify(asset);
        let afa = ensure_account_fungible_asset_object(
            account,
            &asset,
            false
        );

        let fa = fungible_asset::extract(&afa, amount);
        if (fungible_asset::balance(&afa) == 0 && fungible_asset::ungated_transfer_allowed(&afa)) {
            delete_account_fungible_asset_object(account, &asset);
        };
        fa
    }

    /// Ensure fungible asset store exists. If not, create it.
    inline fun ensure_fungible_asset_store(account_address: address) {
        if (!exists<FungibleAssetStore>(account_address)) {
            let account_signer = create_signer::create_signer(account_address);
            move_to(&account_signer, FungibleAssetStore {
                index: smart_table::new()
            })
        }
    }

    /// Get the `AccountFungibleAsset` object of `asset` belonging to `account`.
    /// if `create_on_demand` is true, an default `AccountFungibleAsset` will be created if not exist; otherwise, abort
    /// with error.
    fun get_account_fungible_asset_object(
        account: address,
        asset: &Object<FungibleSource>,
        create_on_demand: bool
    ): Option<Object<AccountFungibleAsset>> acquires FungibleAssetStore {
        ensure_fungible_asset_store(account);
        let asset = fungible_source::verify(asset);
        let index_table = &mut borrow_global_mut<FungibleAssetStore>(account).index;
        if (!smart_table::contains(index_table, copy asset)) {
            if (create_on_demand) {
                let afa_obj = create_account_fungible_asset_object(account, &asset);
                smart_table::add(index_table, copy asset, afa_obj);
            } else {
                return option::none()
            }
        };
        let afa = *smart_table::borrow(index_table, asset);
        option::some(afa)
    }

    /// Ensure the existence and return the `AccountFungibleAsset`.
    inline fun ensure_account_fungible_asset_object(
        account: address,
        asset: &Object<FungibleSource>,
        create_on_demand: bool
    ): Object<AccountFungibleAsset> acquires FungibleAssetStore {
        let afa_opt = get_account_fungible_asset_object(account, asset, create_on_demand);
        assert!(option::is_some(&afa_opt), error::not_found(EACCOUNT_FUNGIBLE_ASSET_OBJECT));
        option::destroy_some(afa_opt)
    }

    /// Create a default `AccountFungibleAsset` object with zero balance of `asset`.
    fun create_account_fungible_asset_object(
        account: address,
        asset: &Object<FungibleSource>
    ): Object<AccountFungibleAsset> {
        // Must review carefully here.
        let asset_signer = create_signer::create_signer(object::object_address(asset));
        let creator_ref = object::create_object_from_object(&asset_signer);
        let afa = fungible_asset::new(&creator_ref, asset);
        // Transfer the owner to `account`.
        object::transfer(&asset_signer, afa, account);
        // Disable transfer of coin object so the object itself never gets transfered.
        let transfer_ref = object::generate_transfer_ref(&creator_ref);
        object::disable_ungated_transfer(&transfer_ref);
        afa
    }

    /// Remove the `AccountFungibleAsset` object of `asset` from `account`.
    fun delete_account_fungible_asset_object(
        account: address,
        asset: &Object<FungibleSource>
    ) acquires FungibleAssetStore {
        // Delete if balance drops to 0 and ungated_transfer is allowed.
        ensure_fungible_asset_store(account);
        let index_table = &mut borrow_global_mut<FungibleAssetStore>(account).index;
        assert!(smart_table::contains(index_table, *asset), error::not_found(EACCOUNT_FUNGIBLE_ASSET_OBJECT));
        let afa = smart_table::remove(index_table, *asset);
        fungible_asset::destory_account_fungible_asset(afa);
    }


    #[test_only]
    use std::signer;

    #[test(creator = @0xcafe, aaron = @0xface)]
    fun test_basic_flow(
        creator: &signer,
        aaron: &signer
    ) acquires FungibleAssetStore {
        let (creator_ref, asset) = fungible_source::create_test_token(creator);
        fungible_source::init_test_fungible_source(&creator_ref);

        let creator_address = signer::address_of(creator);
        let aaron_address = signer::address_of(aaron);

        // Mint
        let fa = fungible_asset::mint(&asset, 100);
        deposit(fa, creator_address);
        assert!(balance(creator_address, &asset) == 100, 1);

        // Transfer
        let fa = withdraw(creator_address, &asset, 80);
        deposit(fa, aaron_address);
        assert!(balance(aaron_address, &asset) == 80, 2);

        assert!(ungated_transfer_allowed(aaron_address, &asset), 3);
        set_ungated_transfer(aaron_address, &asset, false);
        assert!(!ungated_transfer_allowed(aaron_address, &asset), 4);
    }

    #[test(creator = @0xcafe)]
    fun test_empty_account_default_behavior_and_creation_on_demand(
        creator: &signer,
    ) acquires FungibleAssetStore {
        let (creator_ref, asset) = fungible_source::create_test_token(creator);
        fungible_source::init_test_fungible_source(&creator_ref);
        let asset = fungible_source::verify(&asset);
        let creator_address = signer::address_of(creator);

        assert!(balance(creator_address, &asset) == 0, 1);
        assert!(ungated_transfer_allowed(creator_address, &asset), 2);
        assert!(option::is_none(&get_account_fungible_asset_object(creator_address, &asset, false)), 3);
        set_ungated_transfer(creator_address, &asset, false);
        assert!(option::is_some(&get_account_fungible_asset_object(creator_address, &asset, false)), 4);
    }

    #[test(creator = @0xcafe)]
    fun test_auto_deletion(
        creator: &signer,
    ) acquires FungibleAssetStore {
        let (creator_ref, asset) = fungible_source::create_test_token(creator);
        fungible_source::init_test_fungible_source(&creator_ref);
        let asset = fungible_source::verify(&asset);
        let creator_address = signer::address_of(creator);

        // Mint
        let fa = fungible_asset::mint(&asset, 100);
        deposit(fa, creator_address);
        assert!(balance(creator_address, &asset) == 100, 1);
        // exist
        assert!(option::is_some(&get_account_fungible_asset_object(creator_address, &asset, false)), 2);
        let fa = withdraw(creator_address, &asset, 100);
        fungible_asset::burn(fa);
        assert!(balance(creator_address, &asset) == 0, 3);
        assert!(option::is_none(&get_account_fungible_asset_object(creator_address, &asset, false)), 4);
        set_ungated_transfer(creator_address, &asset, true);
        assert!(option::is_some(&get_account_fungible_asset_object(creator_address, &asset, false)), 5);
    }
}
