module fungible_asset::fungible_asset {
    use aptos_std::smart_table::{Self, SmartTable};
    use aptos_framework::object::{Self, Object, object_address, is_owner, generate_transfer_ref, DeleteRef, generate_delete_ref};
    use std::signer;
    use std::error;
    use std::signer::address_of;
    use std::option;
    use std::option::Option;
    #[test_only]
    use aptos_framework::object::ConstructorRef;
    #[test_only]
    use aptos_framework::account::create_account_for_test;

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
    /// The coin object existence error.
    const EFUNGIBLE_ASSET_OBJECT: u64 = 7;
    /// Insufficient amount.
    const EINSUFFICIENT_BALANCE: u64 = 8;
    /// FungibleAsset type mismatch.
    const EFUNGIBLE_ASSET_TYPE_MISMATCH: u64 = 9;


    struct FungibleAssetStore has key {
        index: SmartTable<address, Object<FungibleAsset>>
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct FungibleAsset has key {
        asset_addr: address,
        balance: u64,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct FungibleAssetProperty has key {
        frozen: bool,
        delete_ref: DeleteRef
    }

    // ================================================================================================================
    // Public functions
    // ================================================================================================================

    /// Check the amount of an account.
    public fun balance<T: key>(
        fungible_asset_owner: address,
        fungible_source: &Object<T>
    ): u64 acquires FungibleAssetStore, FungibleAsset {
        let fungible_asset_obj_opt = get_fungible_asset_object(
            fungible_asset_owner,
            object_address(fungible_source),
            false
        );
        if (option::is_none(&fungible_asset_obj_opt)) {
            return 0
        };
        let fungible_asset_obj = option::destroy_some(fungible_asset_obj_opt);
        borrow_global<FungibleAsset>(verify(&fungible_asset_obj)).balance
    }

    /// Check the coin account of `fungible_asset_owner` is frozen or not.
    public fun is_frozen<T: key>(
        fungible_asset_owner: address,
        asset: &Object<T>
    ): bool acquires FungibleAssetStore, FungibleAssetProperty {
        let fungible_asset_obj_opt = get_fungible_asset_object(fungible_asset_owner, object_address(asset), false);
        if (option::is_none(&fungible_asset_obj_opt)) {
            return false
        };
        let fungible_asset_obj = option::destroy_some(fungible_asset_obj_opt);
        borrow_global<FungibleAssetProperty>(verify(&fungible_asset_obj)).frozen
    }


    public fun withdraw<T: key>(
        fungible_asset_owner: &signer,
        asset: &Object<T>,
        amount: u64
    ): FungibleAsset acquires FungibleAssetStore, FungibleAsset, FungibleAssetProperty {
        let account_address = signer::address_of(fungible_asset_owner);
        withdraw_internal(account_address, asset, amount)
    }

    public fun deposit(
        fa: FungibleAsset,
        to: address
    ) acquires FungibleAssetStore, FungibleAsset, FungibleAssetProperty {
        let (stored_fa, property) = borrow_fungible_asset_mut_and_property(
            to,
            fa.asset_addr,
            true /* create token object if not exist */
        );
        assert!(!property.frozen, error::invalid_argument(ESTILL_FROZEN));
        merge(stored_fa, fa);
    }

    // Moves balances around and not the underlying object.
    public fun transfer<T: key>(
        fungible_asset_owner: &signer,
        asset: &Object<T>,
        amount: u64,
        to: address
    ) acquires FungibleAssetStore, FungibleAsset, FungibleAssetProperty {
        // This ensures amount > 0;
        let fa = withdraw(fungible_asset_owner, asset, amount);
        deposit(fa, to);
    }

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

    public(friend) fun set_frozen_flag<T: key>(
        fungible_asset_owner: address,
        asset: &Object<T>,
        frozen: bool
    ) acquires FungibleAssetStore, FungibleAssetProperty {
        let fungible_asset_obj_opt = get_fungible_asset_object(fungible_asset_owner, object_address(asset), true);
        let fungible_asset_obj = option::destroy_some(fungible_asset_obj_opt);
        borrow_global_mut<FungibleAssetProperty>(verify(&fungible_asset_obj)).frozen = frozen;
    }

    public(friend) fun burn(fungible_asset: FungibleAsset) {
        let FungibleAsset {
            asset_addr: _,
            balance: _,
        } = fungible_asset;
    }

    public(friend) fun withdraw_internal<T: key>(
        account: address,
        asset: &Object<T>,
        amount: u64
    ): FungibleAsset acquires FungibleAssetStore, FungibleAsset, FungibleAssetProperty {
        let asset_address = object_address(asset);
        let (stored_fa, property) = borrow_fungible_asset_mut_and_property(
            account,
            asset_address,
            false /* create token object if not exist */
        );

        assert!(!property.frozen, error::invalid_argument(ESTILL_FROZEN));
        let fungible_asset = withdraw_fungible_asset(stored_fa, amount);
        // Clean up token obj if amount drops to 0 and not frozen (verified).
        if (stored_fa.balance == 0) {
            let fungible_asset_obj = remove_fungible_asset_object(account, asset_address);
            let fungible_asset_obj_addr = verify(&fungible_asset_obj);
            let FungibleAsset {
                asset_addr,
                balance: amount,
            } = move_from<FungibleAsset>(fungible_asset_obj_addr);
            let FungibleAssetProperty {
                frozen,
                delete_ref
            } = move_from<FungibleAssetProperty>(fungible_asset_obj_addr);
            assert!(asset_addr == asset_address, error::internal(EASSET_ADDRESS_MISMATCH));
            assert!(amount == 0, error::internal(EBALANCE_NOT_ZERO));
            assert!(!frozen, error::internal(ESTILL_FROZEN));
            object::delete(delete_ref);
        };
        fungible_asset
    }

    /// Ensure the coin store exists. If not, create it.
    fun ensure_fungible_asset_store(account_address: address) {
        if (!exists<FungibleAssetStore>(account_address)) {
            let account_signer = aptos_framework::create_signer::create_signer(account_address);
            move_to(&account_signer, FungibleAssetStore {
                index: smart_table::new()
            })
        }
    }


    fun withdraw_fungible_asset(fa: &mut FungibleAsset, amount: u64): FungibleAsset {
        assert!(amount != 0, error::invalid_argument(EAMOUNT_CANNOT_BE_ZERO));
        assert!(fa.balance >= amount, error::invalid_argument(EINSUFFICIENT_BALANCE));
        fa.balance = fa.balance - amount;
        FungibleAsset {
            asset_addr: fa.asset_addr,
            balance: amount
        }
    }

    fun merge(stored_fa: &mut FungibleAsset, fa: FungibleAsset) {
        let FungibleAsset { asset_addr, balance: amount } = fa;
        // ensure merging the same coin
        assert!(stored_fa.asset_addr == asset_addr, error::invalid_argument(EFUNGIBLE_ASSET_TYPE_MISMATCH));
        stored_fa.balance = stored_fa.balance + amount;
    }

    fun get_fungible_asset_object(
        fungible_asset_owner: address,
        asset_address: address,
        create_on_demand: bool
    ): Option<Object<FungibleAsset>> acquires FungibleAssetStore {
        ensure_fungible_asset_store(fungible_asset_owner);
        let index_table = &mut borrow_global_mut<FungibleAssetStore>(fungible_asset_owner).index;
        if (!smart_table::contains(index_table, asset_address)) {
            if (create_on_demand) {
                let fa_obj = create_fungible_asset_object(fungible_asset_owner, asset_address);
                smart_table::add(index_table, asset_address, fa_obj);
            } else {
                return option::none()
            }
        };
        let fungible_asset_obj = *smart_table::borrow(index_table, asset_address);
        assert!(is_owner(fungible_asset_obj, fungible_asset_owner), error::internal(ENOT_OWNER));
        option::some(fungible_asset_obj)
    }

    /// Create a zero-amount coin object of the passed-in asset.
    fun create_fungible_asset_object(account: address, asset_address: address): Object<FungibleAsset> {
        // Must review carefully here.
        let asset_signer = aptos_framework::create_signer::create_signer(asset_address);
        let creator_ref = object::create_object_from_object(&asset_signer);
        let fungible_asset_signer = object::generate_signer(&creator_ref);
        // Transfer the owner to `account`.
        object::transfer_call(&asset_signer, address_of(&fungible_asset_signer), account);

        // Disable transfer of coin object so the object itself never gets transfered.
        let transfer_ref = generate_transfer_ref(&creator_ref);
        object::disable_ungated_transfer(&transfer_ref);

        move_to(&fungible_asset_signer, FungibleAsset {
            asset_addr: asset_address,
            balance: 0,
        });
        move_to(&fungible_asset_signer, FungibleAssetProperty {
            frozen: false,
            delete_ref: generate_delete_ref(&creator_ref)
        });
        object::object_from_constructor_ref<FungibleAsset>(&creator_ref)
    }

    fun remove_fungible_asset_object(
        fungible_asset_owner: address,
        asset_address: address
    ): Object<FungibleAsset> acquires FungibleAssetStore {
        ensure_fungible_asset_store(fungible_asset_owner);
        let index_table = &mut borrow_global_mut<FungibleAssetStore>(fungible_asset_owner).index;
        assert!(smart_table::contains(index_table, asset_address), error::not_found(EFUNGIBLE_ASSET_OBJECT));
        let fungible_asset_obj = smart_table::remove(index_table, asset_address);
        assert!(is_owner(fungible_asset_obj, fungible_asset_owner), error::internal(ENOT_OWNER));
        fungible_asset_obj
    }

    inline fun borrow_fungible_asset_and_property(
        fungible_asset_owner: address,
        fungible_source_address: address
    ): (&FungibleAsset, &FungibleAssetProperty) acquires FungibleAssetStore, FungibleAsset, FungibleAssetProperty {
        let fungible_asset_obj_opt = get_fungible_asset_object(fungible_asset_owner, fungible_source_address, false);
        assert!(option::is_some(&fungible_asset_obj_opt), error::not_found(EFUNGIBLE_ASSET_OBJECT));
        let fungible_asset_obj = option::destroy_some(fungible_asset_obj_opt);
        let obj_addr = verify(&fungible_asset_obj);
        (borrow_global<FungibleAsset>(obj_addr), borrow_global<FungibleAssetProperty>(obj_addr))
    }

    inline fun borrow_fungible_asset_mut_and_property(
        fungible_asset_owner: address,
        fungible_source_address: address,
        create_on_demand: bool
    ): (&mut FungibleAsset, &FungibleAssetProperty) acquires FungibleAssetStore, FungibleAsset, FungibleAssetProperty {
        let fungible_asset_obj_opt = get_fungible_asset_object(
            fungible_asset_owner,
            fungible_source_address,
            create_on_demand
        );
        assert!(option::is_some(&fungible_asset_obj_opt), error::not_found(EFUNGIBLE_ASSET_OBJECT));
        let fungible_asset_obj = option::destroy_some(fungible_asset_obj_opt);
        let obj_addr = verify(&fungible_asset_obj);
        (borrow_global_mut<FungibleAsset>(obj_addr), borrow_global<FungibleAssetProperty>(obj_addr))
    }

    inline fun verify(fungible_asset_obj: &Object<FungibleAsset>): address {
        let fungible_asset_address = object::object_address(fungible_asset_obj);
        assert!(
            exists<FungibleAsset>(fungible_asset_address),
            error::not_found(EFUNGIBLE_ASSET),
        );
        fungible_asset_address
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
    ) acquires FungibleAssetStore, FungibleAsset, FungibleAssetProperty {
        let (_, asset) = create_test_token(creator);
        let creator_address = signer::address_of(creator);
        let aaron_address = signer::address_of(aaron);

        // Mint
        let fa = mint(object_address(&asset), 100);
        deposit(fa, creator_address);

        // Transfer
        transfer(creator, &asset, 90, aaron_address);
        assert!(balance(creator_address, &asset) == 10, 1);
        assert!(balance(aaron_address, &asset) == 90, 2);

        let fa = withdraw(aaron, &asset, 60);
        deposit(fa, creator_address);
        assert!(balance(creator_address, &asset) == 70, 3);

        let fa_to_burn = withdraw_internal(creator_address, &asset, 70);
        burn(fa_to_burn);
        assert!(balance(creator_address, &asset) == 0, 4);

        // Freeze
        set_frozen_flag(creator_address, &asset, true);
        assert!(is_frozen(creator_address, &asset), 5);
        set_frozen_flag(creator_address, &asset, false);
        assert!(!is_frozen(creator_address, &asset), 6);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 0x10006, location = Self)]
    fun test_failed_withdraw_from_frozen_account(
        creator: &signer,
    ) acquires FungibleAssetStore, FungibleAsset, FungibleAssetProperty {
        let (_, asset) = create_test_token(creator);
        let creator_address = signer::address_of(creator);
        let fa = mint(object_address(&asset), 100);
        deposit(fa, creator_address);

        set_frozen_flag(creator_address, &asset, true);
        let fa = withdraw(creator, &asset, 1);
        burn(fa);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 0x10006, location = Self)]
    fun test_failed_deposit_to_frozen_account(
        creator: &signer,
    ) acquires FungibleAssetStore, FungibleAsset, FungibleAssetProperty {
        let (_, asset) = create_test_token(creator);
        let creator_address = signer::address_of(creator);
        let fa = mint(object_address(&asset), 100);
        set_frozen_flag(creator_address, &asset, true);
        deposit(fa, creator_address);
    }

    #[test(creator = @0xcafe)]
    fun test_empty_account_default_behavior_and_creation_on_demand(
        creator: &signer,
    ) acquires FungibleAssetStore, FungibleAsset, FungibleAssetProperty {
        let (_, asset) = create_test_token(creator);
        let creator_address = signer::address_of(creator);
        assert!(balance(creator_address, &asset) == 0, 1);
        assert!(!is_frozen(creator_address, &asset), 2);
        assert!(option::is_none(&get_fungible_asset_object(creator_address, object_address(&asset), false)), 3);
        assert!(option::is_some(&get_fungible_asset_object(creator_address, object_address(&asset), true)), 3);
    }
}
