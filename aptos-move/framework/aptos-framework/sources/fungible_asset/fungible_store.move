/// This defines a store of `FungibleAssetWallet` under each account.
module aptos_framework::fungible_store {
    use aptos_framework::create_signer;
    use aptos_framework::fungible_asset::{Self, FungibleAssetWallet, FungibleAsset, FungibleAssetMetadata, TransferRef, metadata_from_wallet, BurnRef};
    use aptos_framework::object::{Self, Object};
    use aptos_std::smart_table::{Self, SmartTable};
    use std::error;
    use std::option::{Self, Option};
    use std::signer;
    #[test_only]
    use aptos_framework::fungible_asset::verify;

    /// The fungible asset wallet object existence error.
    const EFUNGIBLE_ASSET_WALLET_OBJECT: u64 = 1;

    /// Represents all the fungible asset wallet objects of an onwer keyed by the base metadata objects.
    struct FungibleAssetStore has key {
        index: SmartTable<Object<FungibleAssetMetadata>, Object<FungibleAssetWallet>>
    }

    /// Check the balance of an account.
    public fun balance<T: key>(
        account: address,
        metadata: &Object<T>
    ): u64 acquires FungibleAssetStore {
        let metadata = fungible_asset::verify(metadata);
        let afa_opt = get_account_fungible_asset_object(
            account,
            &metadata,
            false
        );
        if (option::is_none(&afa_opt)) {
            return 0
        };
        let wallet = option::destroy_some(afa_opt);
        fungible_asset::balance(&wallet)
    }

    /// Return true if `account` allows ungated transfer.
    public fun ungated_transfer_allowed<T: key>(
        account: address,
        metadata: &Object<T>
    ): bool acquires FungibleAssetStore {
        let metadata = fungible_asset::verify(metadata);
        let afa_opt = get_account_fungible_asset_object(
            account,
            &metadata,
            false
        );
        if (option::is_none(&afa_opt)) {
            return true
        };
        let wallet = option::destroy_some(afa_opt);
        fungible_asset::ungated_transfer_allowed(&wallet)
    }

    /// Enable/disable the direct transfer.
    public fun set_ungated_transfer(
        ref: &TransferRef,
        account: address,
        allow: bool
    ) acquires FungibleAssetStore {
        let metadata = fungible_asset::verify(&fungible_asset::transfer_ref_metadata(ref));
        let afa_opt = get_account_fungible_asset_object(account, &metadata, !allow);
        if (option::is_none(&afa_opt)) {
            return
        };
        let wallet = option::destroy_some(afa_opt);
        fungible_asset::set_ungated_transfer(ref, &wallet, allow);
        maybe_delete(wallet);
    }

    /// Withdraw `amount` of fungible asset from `account`.
    public fun withdraw<T: key>(
        account: &signer,
        metadata: &Object<T>,
        amount: u64
    ): FungibleAsset acquires FungibleAssetStore {
        let metadata = fungible_asset::verify(metadata);
        let account_address = signer::address_of(account);
        let wallet = ensure_fungible_asset_wallet(
            account_address,
            &metadata,
            false
        );

        let fa = fungible_asset::withdraw(account, &wallet, amount);
        maybe_delete(wallet);
        fa
    }

    /// Deposit fungible asset to `account`.
    public fun deposit(
        fa: FungibleAsset,
        to: address
    ) acquires FungibleAssetStore {
        let metadata = fungible_asset::metadata_from_asset(&fa);
        let wallet = ensure_fungible_asset_wallet(
            to,
            &metadata,
            true
        );
        fungible_asset::deposit(&wallet, fa);
    }

    /// Transfer `amount` of fungible asset as the owner.
    public fun transfer<T: key>(
        from: &signer,
        metadata: &Object<T>,
        amount: u64,
        to: address
    ) acquires FungibleAssetStore {
        let fa = withdraw(from, metadata, amount);
        deposit(fa, to);
    }

    /// Transfer `ammount` of fungible asset ignoring `allow_ungated_transfer` with `TransferRef`.
    public fun transfer_with_ref(
        ref: &TransferRef,
        from: address,
        to: address,
        amount: u64,
    ) acquires FungibleAssetStore {
        let sender_wallet = ensure_fungible_asset_wallet(
            from,
            &fungible_asset::transfer_ref_metadata(ref),
            false
        );
        let receiver_wallet = ensure_fungible_asset_wallet(
            to,
            &fungible_asset::transfer_ref_metadata(ref),
            true
        );
        fungible_asset::transfer_with_ref(ref, &sender_wallet, &receiver_wallet, amount);
    }

    /// Withdraw `ammount` of fungible asset ignoring `allow_ungated_transfer` with `TransferRef`.
    public fun withdraw_with_ref(
        ref: &TransferRef,
        account: address,
        amount: u64
    ): FungibleAsset acquires FungibleAssetStore {
        let wallet = ensure_fungible_asset_wallet(
            account,
            &fungible_asset::transfer_ref_metadata(ref),
            false
        );
        fungible_asset::withdraw_with_ref(ref, &wallet, amount)
    }

    /// Deposit `ammount` of fungible asset ignoring `allow_ungated_transfer` with `TransferRef`.
    public fun deposit_with_ref(ref: &TransferRef, account: address, fa: FungibleAsset) acquires FungibleAssetStore {
        let wallet = ensure_fungible_asset_wallet(
            account,
            &fungible_asset::transfer_ref_metadata(ref),
            true
        );
        fungible_asset::deposit_with_ref(ref, &wallet, fa);
    }

    /// Burn the `amount` of fungible asset from `account` with `BurnRef`.
    public fun burn(ref: &BurnRef, account: address, amount: u64) acquires FungibleAssetStore {
        let wallet = ensure_fungible_asset_wallet(
            account,
            &fungible_asset::burn_ref_metadata(ref),
            false
        );
        fungible_asset::burn(ref, &wallet, amount);
        maybe_delete(wallet);
    }

    /// Ensure fungible metadata store exists. If not, create it.
    inline fun ensure_fungible_asset_store(account_address: address) {
        if (!exists<FungibleAssetStore>(account_address)) {
            let account_signer = create_signer::create_signer(account_address);
            move_to(&account_signer, FungibleAssetStore {
                index: smart_table::new()
            })
        }
    }

    /// Get the `FungibleAssetWallet` object of `metadata` belonging to `account`.
    /// if `create_on_demand` is true, an default `FungibleAssetWallet` will be created if not exist; otherwise abort.
    fun get_account_fungible_asset_object(
        account: address,
        metadata: &Object<FungibleAssetMetadata>,
        create_on_demand: bool
    ): Option<Object<FungibleAssetWallet>> acquires FungibleAssetStore {
        ensure_fungible_asset_store(account);
        let metadata = fungible_asset::verify(metadata);
        let index_table = &mut borrow_global_mut<FungibleAssetStore>(account).index;
        if (!smart_table::contains(index_table, copy metadata)) {
            if (create_on_demand) {
                let afa_obj = create_account_fungible_asset_object(account, &metadata);
                smart_table::add(index_table, copy metadata, afa_obj);
            } else {
                return option::none()
            }
        };
        let wallet = *smart_table::borrow(index_table, metadata);
        option::some(wallet)
    }

    /// Ensure the existence and return the `FungibleAssetWallet`.
    inline fun ensure_fungible_asset_wallet(
        account: address,
        metadata: &Object<FungibleAssetMetadata>,
        create_on_demand: bool
    ): Object<FungibleAssetWallet> acquires FungibleAssetStore {
        let afa_opt = get_account_fungible_asset_object(account, metadata, create_on_demand);
        assert!(option::is_some(&afa_opt), error::not_found(EFUNGIBLE_ASSET_WALLET_OBJECT));
        option::destroy_some(afa_opt)
    }

    /// Create a default `FungibleAssetWallet` object with zero balance of `metadata`.
    fun create_account_fungible_asset_object(
        account: address,
        metadata: &Object<FungibleAssetMetadata>
    ): Object<FungibleAssetWallet> {
        // Must review carefully here.
        let asset_signer = create_signer::create_signer(object::object_address(metadata));
        let creator_ref = object::create_object_from_object(&asset_signer);
        let wallet = fungible_asset::new_fungible_asset_wallet_object(&creator_ref, metadata);
        // Transfer the owner to `account`.
        object::transfer(&asset_signer, wallet, account);
        // Disable transfer of coin object so the object itself never gets transfered.
        let transfer_ref = object::generate_transfer_ref(&creator_ref);
        object::disable_ungated_transfer(&transfer_ref);
        wallet
    }

    /// Remove the `FungibleAssetWallet` object of `metadata` from `account` if balance drops to 0 and
    /// `allowed_ungated_transfer` is allowed.
    inline fun maybe_delete(wallet: Object<FungibleAssetWallet>) acquires FungibleAssetStore {
        if (fungible_asset::balance(&wallet) == 0 && fungible_asset::ungated_transfer_allowed(&wallet)) {
            let owner = object::owner(wallet);
            let index_table = &mut borrow_global_mut<FungibleAssetStore>(owner).index;
            smart_table::remove(index_table, metadata_from_wallet(&wallet));
            fungible_asset::destory_fungible_asset_wallet(wallet);
        };
    }

    #[test(creator = @0xcafe, aaron = @0xface)]
    fun test_basic_flow(
        creator: &signer,
        aaron: &signer
    ) acquires FungibleAssetStore {
        let (mint_ref, transfer_ref, burn_ref, metadata) = fungible_asset::generate_refs(creator);

        let creator_address = signer::address_of(creator);
        let aaron_address = signer::address_of(aaron);

        // Mint
        let fa = fungible_asset::mint(&mint_ref, 100);
        deposit(fa, creator_address);
        assert!(balance(creator_address, &metadata) == 100, 1);

        // Transfer
        let fa = withdraw(creator, &metadata, 80);
        deposit(fa, aaron_address);
        assert!(balance(aaron_address, &metadata) == 80, 2);

        assert!(ungated_transfer_allowed(aaron_address, &metadata), 3);
        set_ungated_transfer(&transfer_ref, aaron_address, false);
        assert!(!ungated_transfer_allowed(aaron_address, &metadata), 4);
        let fa = withdraw_with_ref(&transfer_ref, aaron_address, 20);
        deposit_with_ref(&transfer_ref, aaron_address, fa);
        transfer_with_ref(&transfer_ref, aaron_address, creator_address, 20);
        assert!(balance(creator_address, &metadata) == 40, 5);

        // burn
        burn(&burn_ref, creator_address, 30);
        assert!(fungible_asset::supply(&metadata) == 70, 6);
    }

    #[test(creator = @0xcafe)]
    fun test_empty_account_default_behavior_and_creation_on_demand(
        creator: &signer,
    ) acquires FungibleAssetStore {
        let (_mint_ref, transfer_ref, _burn_ref, metadata) = fungible_asset::generate_refs(creator);
        let metadata = verify((&metadata));
        let creator_address = signer::address_of(creator);

        assert!(balance(creator_address, &metadata) == 0, 1);
        assert!(ungated_transfer_allowed(creator_address, &metadata), 2);
        assert!(option::is_none(&get_account_fungible_asset_object(creator_address, &metadata, false)), 3);
        set_ungated_transfer(&transfer_ref, creator_address, false);
        assert!(option::is_some(&get_account_fungible_asset_object(creator_address, &metadata, false)), 4);
    }

    #[test(creator = @0xcafe)]
    fun test_auto_deletion(
        creator: &signer,
    ) acquires FungibleAssetStore {
        let (mint_ref, transfer_ref, burn_ref, metadata) = fungible_asset::generate_refs(creator);
        let metadata = verify((&metadata));
        let creator_address = signer::address_of(creator);

        // Mint
        let fa = fungible_asset::mint(&mint_ref, 100);
        deposit(fa, creator_address);
        assert!(balance(creator_address, &metadata) == 100, 1);
        // exist
        assert!(option::is_some(&get_account_fungible_asset_object(creator_address, &metadata, false)), 2);

        burn(&burn_ref, creator_address, 100);
        assert!(balance(creator_address, &metadata) == 0, 3);
        assert!(option::is_none(&get_account_fungible_asset_object(creator_address, &metadata, false)), 4);
        set_ungated_transfer(&transfer_ref, creator_address, false);
        assert!(option::is_some(&get_account_fungible_asset_object(creator_address, &metadata, false)), 5);
        set_ungated_transfer(&transfer_ref, creator_address, true);
        assert!(option::is_none(&get_account_fungible_asset_object(creator_address, &metadata, false)), 6);
    }
}
