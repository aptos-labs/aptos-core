/// This defines the fungible asset module that can issue fungible assets of any `FungibleSource` object. The source
/// can be a token object or any object that equipped with `FungibleSource` resource.
module aptos_framework::fungible_asset {
    use aptos_framework::object::{Self, Object, DeleteRef, ConstructorRef};
    use aptos_framework::fungible_source::{Self, FungibleSource};
    use std::error;
    #[test_only]
    use aptos_framework::fungible_source::init_test_fungible_source;

    friend aptos_framework::fungible_caps;
    friend aptos_framework::fungible_store;

    /// Amount cannot be zero.
    const EAMOUNT_CANNOT_BE_ZERO: u64 = 1;
    /// The token account has positive amount so cannot be deleted.
    const EBALANCE_NOT_ZERO: u64 = 2;
    /// The token account is still allow_ungated_transfer so cannot be deleted.
    const EUNGATED_TRANSFER_IS_NOT_ALLOWED: u64 = 3;
    /// Insufficient amount.
    const EINSUFFICIENT_BALANCE: u64 = 4;
    /// FungibleAsset type mismatch.
    const EFUNGIBLE_ASSET_TYPE_MISMATCH: u64 = 5;


    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// The resource of an object recording the properties of the fungible assets held of the object owner.
    struct AccountFungibleAsset has key {
        /// The address of the base asset object.
        asset: Object<FungibleSource>,
        /// The balance of the fungible asset.
        balance: u64,
        /// Fungible Assets transferring is a common operation, this allows for disabling and enabling
        /// transfers bypassing the use of a TransferCap.
        allow_ungated_transfer: bool,
        /// The delete_ref of this object, used for cleanup.
        delete_ref: DeleteRef
    }

    /// The transferable version of fungible asset.
    /// Note: it does not have `store` ability so only used in hot potato pattern.
    struct FungibleAsset {
        asset: Object<FungibleSource>,
        amount: u64,
    }

    /// Return the underlying fungible source.
    public fun fungible_asset_source(fa: &FungibleAsset): Object<FungibleSource> {
        fa.asset
    }

    /// Return the amount.
    public fun fungible_asset_amount(fa: &FungibleAsset): u64 {
        fa.amount
    }

    /// Create a new `Object<AccountFungibleAsset>`.
    public(friend) fun new<T: key>(
        creator_ref: &ConstructorRef,
        asset: &Object<T>,
    ): Object<AccountFungibleAsset> {
        let pfa_signer = object::generate_signer(creator_ref);
        let asset = fungible_source::verify(asset);

        move_to(&pfa_signer, AccountFungibleAsset {
            asset,
            balance: 0,
            allow_ungated_transfer: true,
            delete_ref: object::generate_delete_ref(creator_ref)
        });
        object::object_from_constructor_ref<AccountFungibleAsset>(creator_ref)
    }

    /// Mint fungible asset with `amount`.
    public(friend) fun mint<T: key>(
        asset: &Object<T>,
        amount: u64,
    ): FungibleAsset {
        assert!(amount != 0, error::invalid_argument(EAMOUNT_CANNOT_BE_ZERO));
        let asset = fungible_source::verify(asset);
        fungible_source::increase_supply(&asset, amount);
        FungibleAsset {
            asset,
            amount
        }
    }

    /// Desotry `AccountFungibleAsset` object.
    public(friend) fun destory_account_fungible_asset(afa: Object<AccountFungibleAsset>) acquires AccountFungibleAsset {
        let AccountFungibleAsset {
            asset: _,
            balance: _,
            allow_ungated_transfer: _,
            delete_ref
        } = move_from<AccountFungibleAsset>(object::object_address(&afa));
        object::delete(delete_ref);
    }

    /// Burn fungible asset.
    public(friend) fun burn(fungible_asset: FungibleAsset) {
        let FungibleAsset {
            asset,
            amount,
        } = fungible_asset;
        fungible_source::decrease_supply(&asset, amount);
    }

    /// Extract `amount` of fungible asset from a `AccountFungibleAsset`.
    public(friend) fun extract(
        afa: &Object<AccountFungibleAsset>,
        amount: u64,
    ): FungibleAsset acquires AccountFungibleAsset {
        assert!(amount != 0, error::invalid_argument(EAMOUNT_CANNOT_BE_ZERO));
        let afa = borrow_fungible_asset_mut(afa);
        assert!(afa.allow_ungated_transfer, error::invalid_argument(EUNGATED_TRANSFER_IS_NOT_ALLOWED));
        assert!(afa.balance >= amount, error::invalid_argument(EINSUFFICIENT_BALANCE));
        afa.balance = afa.balance - amount;
        FungibleAsset {
            asset: afa.asset,
            amount
        }
    }

    /// Merge `amount` of fungible asset to `AccountFungibleAsset`.
    public(friend) fun merge(
        afa: &Object<AccountFungibleAsset>,
        fa: FungibleAsset,
    ) acquires AccountFungibleAsset {
        let FungibleAsset { asset, amount } = fa;
        // ensure merging the same coin
        let afa = borrow_fungible_asset_mut(afa);
        assert!(afa.allow_ungated_transfer, error::invalid_argument(EUNGATED_TRANSFER_IS_NOT_ALLOWED));
        assert!(afa.asset == asset, error::invalid_argument(EFUNGIBLE_ASSET_TYPE_MISMATCH));
        afa.balance = afa.balance + amount;
    }


    /// Get the balance of an `Object<AccountFungibleAsset>`.
    public fun balance(afa: &Object<AccountFungibleAsset>): u64 acquires AccountFungibleAsset {
        borrow_fungible_asset(afa).balance
    }

    /// Get the source object of an `Object<AccountFungibleAsset>`.
    public fun account_fungible_asset_source(
        afa: &Object<AccountFungibleAsset>
    ): Object<FungibleSource> acquires AccountFungibleAsset {
        borrow_fungible_asset(afa).asset
    }

    /// Return if `ungated_transfer` is allowed.
    public fun ungated_transfer_allowed(afa: &Object<AccountFungibleAsset>): bool acquires AccountFungibleAsset {
        borrow_fungible_asset(afa).allow_ungated_transfer
    }

    /// Set `allow_ungated_transfer`.
    public(friend) fun set_ungated_transfer(
        afa: &Object<AccountFungibleAsset>,
        allow: bool
    ) acquires AccountFungibleAsset {
        borrow_fungible_asset_mut(afa).allow_ungated_transfer = allow;
    }

    /// Private helper funtion to get an immutable reference of the `AccountFungibleAsset`.
    inline fun borrow_fungible_asset(
        pfa: &Object<AccountFungibleAsset>,
    ): &AccountFungibleAsset acquires AccountFungibleAsset {
        borrow_global<AccountFungibleAsset>(object::object_address(pfa))
    }

    /// Private helper funtion to get a mutable reference of the `AccountFungibleAsset`.
    inline fun borrow_fungible_asset_mut(
        pfa: &Object<AccountFungibleAsset>,
    ): &mut AccountFungibleAsset acquires AccountFungibleAsset {
        borrow_global_mut<AccountFungibleAsset>(object::object_address(pfa))
    }

    #[test_only]
    fun generate_asset_and_account_fungible_asset(
        creator: &signer
    ): (Object<FungibleSource>, Object<AccountFungibleAsset>) {
        let (creator_ref, asset) = fungible_source::create_test_token(creator);
        init_test_fungible_source(&creator_ref);
        let asset = fungible_source::verify(&asset);

        let asset_signer = object::generate_signer(&creator_ref);
        let afa_ref = object::create_object_from_object(&asset_signer);
        let afa = new(&afa_ref, &asset);
        (asset, afa)
    }

    #[test(creator = @0xcafe)]
    fun test_basic_flow(
        creator: &signer,
    ) acquires AccountFungibleAsset {
        let (asset, afa) = generate_asset_and_account_fungible_asset(creator);

        assert!(fungible_source::get_current_supply(&asset) == 0, 1);
        // Mint
        let fa = mint(&asset, 100);
        assert!(fungible_source::get_current_supply(&asset) == 100, 2);
        // Merge
        merge(&afa, fa);
        // Extract
        let fa = extract(&afa, 80);
        assert!(fungible_source::get_current_supply(&asset) == 100, 3);
        // burn
        burn(fa);
        assert!(fungible_source::get_current_supply(&asset) == 20, 4);
        assert!(balance(&afa) == 20, 5);

        // Ungated transfer
        assert!(ungated_transfer_allowed(&afa), 6);
        set_ungated_transfer(&afa, false);
        assert!(!ungated_transfer_allowed(&afa), 7);

        destory_account_fungible_asset(afa);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 0x10003, location = Self)]
    fun test_failed_extract(
        creator: &signer,
    ) acquires AccountFungibleAsset {
        let (asset, afa) = generate_asset_and_account_fungible_asset(creator);
        let fa = mint(&asset, 10);
        merge(&afa, fa);
        set_ungated_transfer(&afa, false);
        let fa = extract(&afa, 1);
        burn(fa);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 0x10003, location = Self)]
    fun test_failed_merge(
        creator: &signer,
    ) acquires AccountFungibleAsset {
        let (asset, afa) = generate_asset_and_account_fungible_asset(creator);
        let fa = mint(&asset, 10);
        merge(&afa, fa);
        set_ungated_transfer(&afa, false);
        let fa = extract(&afa, 1);
        burn(fa);
    }
}
