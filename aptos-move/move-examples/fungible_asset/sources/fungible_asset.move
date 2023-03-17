/// This defines the fungible asset module that can issue fungible assets of any `FungibleSource` object. The source
/// can be a token object or any object that equipped with `FungibleSource` resource.
module fungible_asset::fungible_asset {
    use aptos_std::smart_table::{Self, SmartTable};
    use aptos_framework::object::{Self, Object, object_address, generate_transfer_ref, DeleteRef, generate_delete_ref};
    use std::error;
    use std::signer::address_of;
    use std::option;
    use std::option::Option;
    #[test_only]
    use aptos_framework::object::ConstructorRef;
    #[test_only]
    use aptos_framework::account::create_account_for_test;
    #[test_only]
    use std::signer;

    friend fungible_asset::fungible_source;

    /// The coin resource existence error.
    const EFUNGIBLE_ASSET: u64 = 1;
    /// Amount cannot be zero.
    const EAMOUNT_CANNOT_BE_ZERO: u64 = 2;
    /// Not the owner.
    const ENOT_OWNER: u64 = 3;
    /// The token account has asset mismatch.
    const EASSET_ADDRESS_MISMATCH: u64 = 4;
    /// The token account has positive amount so cannot be deleted.
    const EBALANCE_NOT_ZERO: u64 = 5;
    /// The token account is still frozen so cannot be deleted.
    const ESTILL_FROZEN: u64 = 6;
    /// The pinned fungible asset object existence error.
    const PINNED_EFUNGIBLE_ASSET_OBJECT: u64 = 7;
    /// Insufficient amount.
    const EINSUFFICIENT_BALANCE: u64 = 8;
    /// FungibleAsset type mismatch.
    const EFUNGIBLE_ASSET_TYPE_MISMATCH: u64 = 9;


    /// Represents all the fungible asset objects of an onwer keyed by the address of the base asset object.
    struct FungibleAssetStore has key {
        index: SmartTable<address, Object<PinnedFungibleAsset>>
    }

    /// The the pinned fungible asset the object of which cannot be transferred.
    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct PinnedFungibleAsset has key {
        /// The address of the base asset object.
        asset_addr: address,
        /// The balance of the fungible asset.
        balance: u64,
        /// Whether this account is frozen for this fungible asset.
        frozen: bool,
        /// The delete_ref of this object, used for cleanup.
        delete_ref: DeleteRef
    }

    /// The unpinned version of fungible asset.
    /// Note: it does not have `store` ability so only used in hot potato pattern.
    struct FungibleAsset {
        asset_addr: address,
        balance: u64,
    }

    /// Check the amount of an account.
    public fun balance<T: key>(
        fungible_asset_owner: address,
        fungible_source: &Object<T>
    ): u64 acquires FungibleAssetStore, PinnedFungibleAsset {
        let pfa_opt = get_pinned_fungible_asset_object(
            fungible_asset_owner,
            object_address(fungible_source),
            false
        );
        if (option::is_none(&pfa_opt)) {
            return 0
        };
        let pfa = option::destroy_some(pfa_opt);
        borrow_global<PinnedFungibleAsset>(object_address(&pfa)).balance
    }

    /// Check the coin account of `fungible_asset_owner` is frozen or not.
    public fun is_frozen<T: key>(
        fungible_asset_owner: address,
        fungible_source: &Object<T>
    ): bool acquires FungibleAssetStore, PinnedFungibleAsset {
        let pfa_opt = get_pinned_fungible_asset_object(
            fungible_asset_owner,
            object_address(fungible_source),
            false
        );
        if (option::is_none(&pfa_opt)) {
            return false
        };
        let pfa = option::destroy_some(pfa_opt);
        borrow_global<PinnedFungibleAsset>(object_address(&pfa)).frozen
    }

    /// Deposit fungible asset to an account.
    public fun deposit(
        fa: FungibleAsset,
        to: address
    ) acquires FungibleAssetStore, PinnedFungibleAsset {
        let pfa = borrow_fungible_asset_mut(
            to,
            fa.asset_addr,
            true /* create token object if not exist */
        );
        assert!(!pfa.frozen, error::invalid_argument(ESTILL_FROZEN));
        merge(pfa, fa);
    }

    /// Mint fungible asset with `amount`.
    public(friend) fun mint(
        asset_addr: address,
        amount: u64,
    ): FungibleAsset {
        assert!(amount != 0, error::invalid_argument(EAMOUNT_CANNOT_BE_ZERO));
        FungibleAsset {
            asset_addr,
            balance: amount
        }
    }

    /// Freeeze/unfreeze any account of asset address `asset_addr`.
    public(friend) fun set_frozen_flag(
        fungible_asset_owner: address,
        asset_addr: address,
        frozen: bool
    ) acquires FungibleAssetStore, PinnedFungibleAsset {
        let pfa_opt = get_pinned_fungible_asset_object(fungible_asset_owner, asset_addr, true);
        let pfa = option::destroy_some(pfa_opt);
        borrow_global_mut<PinnedFungibleAsset>(object_address(&pfa)).frozen = frozen;
    }

    /// Burn fungible asset.
    public(friend) fun burn(fungible_asset: FungibleAsset) {
        let FungibleAsset {
            asset_addr: _,
            balance: _,
        } = fungible_asset;
    }

    /// Withdraw `amount` of fungible asset of asset address `asset_addr` from `account`.
    public(friend) fun withdraw(
        account: address,
        asset_addr: address,
        amount: u64
    ): FungibleAsset acquires FungibleAssetStore, PinnedFungibleAsset {
        let pfa = borrow_fungible_asset_mut(
            account,
            asset_addr,
            false /* create token object if not exist */
        );

        assert!(!pfa.frozen, error::invalid_argument(ESTILL_FROZEN));
        let fungible_asset = extract(pfa, amount);
        // Clean up token obj if amount drops to 0 and not frozen (verified).
        if (pfa.balance == 0) {
            let pfa = remove_pinned_fungible_asset_object(account, asset_addr);
            let PinnedFungibleAsset {
                asset_addr: _,
                balance: _,
                frozen: _,
                delete_ref
            } = move_from<PinnedFungibleAsset>(object_address(&pfa));
            object::delete(delete_ref);
        };
        fungible_asset
    }

    /// Ensure the coin store exists. If not, create it.
    inline fun ensure_fungible_asset_store(account_address: address) {
        if (!exists<FungibleAssetStore>(account_address)) {
            let account_signer = aptos_framework::create_signer::create_signer(account_address);
            move_to(&account_signer, FungibleAssetStore {
                index: smart_table::new()
            })
        }
    }


    /// Extract `amount` of fungible asset from a `PinnedFungibleAsset`.
    fun extract(fa: &mut PinnedFungibleAsset, amount: u64): FungibleAsset {
        assert!(amount != 0, error::invalid_argument(EAMOUNT_CANNOT_BE_ZERO));
        assert!(fa.balance >= amount, error::invalid_argument(EINSUFFICIENT_BALANCE));
        fa.balance = fa.balance - amount;
        FungibleAsset {
            asset_addr: fa.asset_addr,
            balance: amount
        }
    }

    /// Merge `amount` of fungible asset to `PinnedFungibleAsset`.
    fun merge(pfa: &mut PinnedFungibleAsset, fa: FungibleAsset) {
        let FungibleAsset { asset_addr, balance: amount } = fa;
        // ensure merging the same coin
        assert!(pfa.asset_addr == asset_addr, error::invalid_argument(EFUNGIBLE_ASSET_TYPE_MISMATCH));
        pfa.balance = pfa.balance + amount;
    }

    /// Get the `PinnedFungibleAsset` object of an asset from owner address.
    /// if `create_on_demand` is true, an default`PinnedFungibleAsset` will be created if not exists; otherwise, abort
    /// with error.
    fun get_pinned_fungible_asset_object(
        fungible_asset_owner: address,
        asset_address: address,
        create_on_demand: bool
    ): Option<Object<PinnedFungibleAsset>> acquires FungibleAssetStore {
        ensure_fungible_asset_store(fungible_asset_owner);
        let index_table = &mut borrow_global_mut<FungibleAssetStore>(fungible_asset_owner).index;
        if (!smart_table::contains(index_table, asset_address)) {
            if (create_on_demand) {
                let pfa_obj = create_pinned_fungible_asset_object(fungible_asset_owner, asset_address);
                smart_table::add(index_table, asset_address, pfa_obj);
            } else {
                return option::none()
            }
        };
        let pfa = *smart_table::borrow(index_table, asset_address);
        option::some(pfa)
    }

    /// Create a default `PinnedFungibleAsset` object with zero balance of the passed-in asset.
    fun create_pinned_fungible_asset_object(account: address, asset_address: address): Object<PinnedFungibleAsset> {
        // Must review carefully here.
        let asset_signer = aptos_framework::create_signer::create_signer(asset_address);
        let creator_ref = object::create_object_from_object(&asset_signer);
        let pfa_signer = object::generate_signer(&creator_ref);
        // Transfer the owner to `account`.
        object::transfer_call(&asset_signer, address_of(&pfa_signer), account);

        // Disable transfer of coin object so the object itself never gets transfered.
        let transfer_ref = generate_transfer_ref(&creator_ref);
        object::disable_ungated_transfer(&transfer_ref);

        move_to(&pfa_signer, PinnedFungibleAsset {
            asset_addr: asset_address,
            balance: 0,
            frozen: false,
            delete_ref: generate_delete_ref(&creator_ref)
        });
        object::object_from_constructor_ref<PinnedFungibleAsset>(&creator_ref)
    }

    /// Remove the corresponding `PinnedFungibleAsset` object from the index of owner.
    fun remove_pinned_fungible_asset_object(
        fungible_asset_owner: address,
        asset_address: address
    ): Object<PinnedFungibleAsset> acquires FungibleAssetStore {
        ensure_fungible_asset_store(fungible_asset_owner);
        let index_table = &mut borrow_global_mut<FungibleAssetStore>(fungible_asset_owner).index;
        assert!(smart_table::contains(index_table, asset_address), error::not_found(PINNED_EFUNGIBLE_ASSET_OBJECT));
        smart_table::remove(index_table, asset_address)
    }

    /// Private helper funtion to get an immutable reference of the `PinnedFungibleAsset` specified by `asset_address`.
    inline fun borrow_fungible_asset(
        fungible_asset_owner: address,
        asset_address: address
    ): &PinnedFungibleAsset acquires FungibleAssetStore, PinnedFungibleAsset {
        let pfa_opt = get_pinned_fungible_asset_object(fungible_asset_owner, asset_address, false);
        assert!(option::is_some(&pfa_opt), error::not_found(PINNED_EFUNGIBLE_ASSET_OBJECT));
        let pfa = option::destroy_some(pfa_opt);
        borrow_global<PinnedFungibleAsset>(object_address(&pfa))
    }

    /// Private helper funtion to get a mutable reference of the `PinnedFungibleAsset` specified by `asset_address`.
    inline fun borrow_fungible_asset_mut(
        fungible_asset_owner: address,
        asset_address: address,
        create_on_demand: bool
    ): &mut PinnedFungibleAsset acquires FungibleAssetStore, PinnedFungibleAsset {
        let pfa_opt = get_pinned_fungible_asset_object(
            fungible_asset_owner,
            asset_address,
            create_on_demand
        );
        assert!(option::is_some(&pfa_opt), error::not_found(PINNED_EFUNGIBLE_ASSET_OBJECT));
        let pfa = option::destroy_some(pfa_opt);
        borrow_global_mut<PinnedFungibleAsset>(object_address(&pfa))
    }

    #[test_only]
    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct TestToken has key {}

    #[test_only]
    public fun create_test_token(creator: &signer): (ConstructorRef, Object<TestToken>) {
        create_account_for_test(address_of(creator));
        let creator_ref = object::create_object_from_account(creator);
        let object_signer = object::generate_signer(&creator_ref);
        move_to(&object_signer, TestToken {});

        let token = object::object_from_constructor_ref<TestToken>(&creator_ref);
        (creator_ref, token)
    }

    #[test(creator = @0xcafe, aaron = @0xface)]
    fun test_basic_flow(
        creator: &signer,
        aaron: &signer
    ) acquires FungibleAssetStore, PinnedFungibleAsset {
        let (_, asset) = create_test_token(creator);
        let creator_address = signer::address_of(creator);
        let aaron_address = signer::address_of(aaron);
        let asset_addr = object_address(&asset);

        // Mint
        let fa = mint(asset_addr, 100);
        deposit(fa, creator_address);

        // Transfer
        let fa = withdraw(creator_address, asset_addr, 90);
        deposit(fa, aaron_address);
        assert!(balance(creator_address, &asset) == 10, 1);
        assert!(balance(aaron_address, &asset) == 90, 2);

        let fa = withdraw(aaron_address, asset_addr, 60);
        deposit(fa, creator_address);
        assert!(balance(creator_address, &asset) == 70, 3);

        let fa_to_burn = withdraw(creator_address, asset_addr, 70);
        burn(fa_to_burn);
        assert!(balance(creator_address, &asset) == 0, 4);

        // Freeze
        set_frozen_flag(creator_address, asset_addr, true);
        assert!(is_frozen(creator_address, &asset), 5);
        set_frozen_flag(creator_address, asset_addr, false);
        assert!(!is_frozen(creator_address, &asset), 6);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 0x10006, location = Self)]
    fun test_failed_withdraw_from_frozen_account(
        creator: &signer,
    ) acquires FungibleAssetStore, PinnedFungibleAsset {
        let (_, asset) = create_test_token(creator);
        let creator_address = signer::address_of(creator);
        let asset_addr = object_address(&asset);

        let fa = mint(object_address(&asset), 100);
        deposit(fa, creator_address);

        set_frozen_flag(creator_address, asset_addr, true);
        let fa = withdraw(creator_address, asset_addr, 1);
        burn(fa);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 0x10006, location = Self)]
    fun test_failed_deposit_to_frozen_account(
        creator: &signer,
    ) acquires FungibleAssetStore, PinnedFungibleAsset {
        let (_, asset) = create_test_token(creator);
        let creator_address = signer::address_of(creator);
        let asset_addr = object_address(&asset);

        let fa = mint(asset_addr, 100);
        set_frozen_flag(creator_address, asset_addr, true);
        deposit(fa, creator_address);
    }

    #[test(creator = @0xcafe)]
    fun test_empty_account_default_behavior_and_creation_on_demand(
        creator: &signer,
    ) acquires FungibleAssetStore, PinnedFungibleAsset {
        let (_, asset) = create_test_token(creator);
        let creator_address = signer::address_of(creator);
        let asset_addr = object_address(&asset);

        assert!(balance(creator_address, &asset) == 0, 1);
        assert!(!is_frozen(creator_address, &asset), 2);
        assert!(option::is_none(&get_pinned_fungible_asset_object(creator_address, asset_addr, false)), 3);
        assert!(option::is_some(&get_pinned_fungible_asset_object(creator_address, asset_addr, true)), 3);
    }
}
