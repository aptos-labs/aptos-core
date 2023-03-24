/// This defines the fungible asset module that can issue fungible asset of any `FungibleAssetMetadata` object. The
/// metadata object can be any object that equipped with `FungibleAssetMetadata` resource.
module aptos_framework::fungible_asset {
    use aptos_framework::object::{Self, Object, DeleteRef, ConstructorRef};
    use std::error;
    use std::option::{Self, Option};
    use std::string::String;
    use std::signer;

    friend aptos_framework::fungible_store;

    /// Amount cannot be zero.
    const EAMOUNT_CANNOT_BE_ZERO: u64 = 1;
    /// The transfer ref and the wallet do not match.
    const ETRANSFER_REF_AND_WALLET_MISMATCH: u64 = 2;
    /// The burn ref and the the wallet do not match.
    const EBURN_REF_AND_WALLET_MISMATCH: u64 = 3;
    /// The token account is still allow_ungated_transfer so cannot be deleted.
    const EUNGATED_TRANSFER_IS_NOT_ALLOWED: u64 = 4;
    /// Insufficient amount.
    const EINSUFFICIENT_BALANCE: u64 = 5;
    /// FungibleAsset type and the wallet type mismatch.
    const EFUNGIBLE_ASSET_AND_WALLET_MISMATCH: u64 = 6;
    /// Amount cannot be zero.
    const EZERO_AMOUNT: u64 = 7;
    /// Current supply overflow
    const ECURRENT_SUPPLY_OVERFLOW: u64 = 8;
    /// Current supply underflow
    const ECURRENT_SUPPLY_UNDERFLOW: u64 = 9;
    /// Not the owner,
    const ENOT_OWNER: u64 = 10;

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Define the metadata required of an metadata to be fungible.
    struct FungibleAssetMetadata has key {
        /// The current supply.
        supply: u64,
        /// The maximum supply limit where `option::none()` means no limit.
        maximum: Option<u64>,
        /// Name of the fungible metadata, i.e., "USDT".
        name: String,
        /// Symbol of the fungible metadata, usually a shorter version of the name.
        /// For example, Singapore Dollar is SGD.
        symbol: String,
        /// Number of decimals used to get its user representation.
        /// For example, if `decimals` equals `2`, a balance of `505` coins should
        /// be displayed to a user as `5.05` (`505 / 10 ** 2`).
        decimals: u8,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// The resource of an object holding the properties of fungible assets.
    struct FungibleAssetWallet has key {
        /// The address of the base metadata object.
        metadata: Object<FungibleAssetMetadata>,
        /// The balance of the fungible metadata.
        balance: u64,
        /// Fungible Assets transferring is a common operation, this allows for disabling and enabling
        /// transfers bypassing the use of a TransferRef.
        allow_ungated_transfer: bool,
        /// The delete_ref of this object, used for cleanup.
        delete_ref: DeleteRef
    }

    /// The transferable version of fungible metadata.
    /// Note: it does not have `store` ability, only used in hot potato pattern.
    struct FungibleAsset {
        metadata: Object<FungibleAssetMetadata>,
        amount: u64,
    }

    /// Ref to mint.
    struct MintRef has drop, store {
        metadata: Object<FungibleAssetMetadata>
    }

    /// Ref to control the transfer.
    struct TransferRef has drop, store {
        metadata: Object<FungibleAssetMetadata>
    }

    /// Ref to burn..
    struct BurnRef has drop, store {
        metadata: Object<FungibleAssetMetadata>
    }

    /// The initialization of an object with `FungibleAssetMetadata`.
    public fun init_metadata(
        constructor_ref: &ConstructorRef,
        maximum_supply: u64,
        name: String,
        symbol: String,
        decimals: u8,
    ): (MintRef, TransferRef, BurnRef) {
        let metadata_object_signer = object::generate_signer(constructor_ref);
        let converted_maximum = if (maximum_supply == 0) {
            option::none()
        } else {
            option::some(maximum_supply)
        };
        move_to(&metadata_object_signer,
            FungibleAssetMetadata {
                supply: 0,
                maximum: converted_maximum,
                name,
                symbol,
                decimals,
            }
        );
        let metadata = object::object_from_constructor_ref<FungibleAssetMetadata>(constructor_ref);
        (MintRef { metadata }, TransferRef { metadata }, BurnRef { metadata })
    }

    /// Get the current supply from `metadata`.
    public fun supply<T: key>(metadata: &Object<T>): u64 acquires FungibleAssetMetadata {
        borrow_fungible_metadata(metadata).supply
    }

    /// Get the maximum supply from `metadata`.
    public fun maximum<T: key>(metadata: &Object<T>): Option<u64> acquires FungibleAssetMetadata {
        borrow_fungible_metadata(metadata).maximum
    }

    /// Get the name of the fungible asset from `metadata`.
    public fun name<T: key>(metadata: &Object<T>): String acquires FungibleAssetMetadata {
        borrow_fungible_metadata(metadata).name
    }

    /// Get the symbol of the fungible asset from `metadata`.
    public fun symbol<T: key>(metadata: &Object<T>): String acquires FungibleAssetMetadata {
        borrow_fungible_metadata(metadata).symbol
    }

    /// Get the decimals from `metadata`.
    public fun decimals<T: key>(metadata: &Object<T>): u8 acquires FungibleAssetMetadata {
        borrow_fungible_metadata(metadata).decimals
    }


    /// Verify any object is equipped with `FungibleAssetMetadata` and return the object.
    public fun verify<T: key>(metadata: &Object<T>): Object<FungibleAssetMetadata> {
        let addr = object::object_address(metadata);
        object::address_to_object<FungibleAssetMetadata>(addr)
    }

    /// Create a new wallet object to hold fungible asset.
    public(friend) fun new_fungible_asset_wallet_object<T: key>(
        creator_ref: &ConstructorRef,
        metadata: &Object<T>,
    ): Object<FungibleAssetWallet> {
        let wallet_signer = object::generate_signer(creator_ref);
        let metadata = verify(metadata);

        move_to(&wallet_signer, FungibleAssetWallet {
            metadata,
            balance: 0,
            allow_ungated_transfer: true,
            delete_ref: object::generate_delete_ref(creator_ref)
        });
        object::object_from_constructor_ref<FungibleAssetWallet>(creator_ref)
    }

    /// Return the underlying metadata object
    public fun metadata_from_asset(fa: &FungibleAsset): Object<FungibleAssetMetadata> {
        fa.metadata
    }

    /// Return the underlying metadata object.
    public fun metadata_from_wallet(
        wallet: &Object<FungibleAssetWallet>
    ): Object<FungibleAssetMetadata> acquires FungibleAssetWallet {
        borrow_fungible_asset(wallet).metadata
    }

    /// Return `amount` inside.
    public fun amount(fa: &FungibleAsset): u64 {
        fa.amount
    }

    /// Destroy `wallet` object.
    public(friend) fun destory_fungible_asset_wallet(
        wallet: Object<FungibleAssetWallet>
    ) acquires FungibleAssetWallet {
        let FungibleAssetWallet {
            metadata: _,
            balance: _,
            allow_ungated_transfer: _,
            delete_ref
        } = move_from<FungibleAssetWallet>(object::object_address(&wallet));
        object::delete(delete_ref);
    }

    /// Get the balance of `wallet`.
    public fun balance(wallet: &Object<FungibleAssetWallet>): u64 acquires FungibleAssetWallet {
        borrow_fungible_asset(wallet).balance
    }


    /// Return `allowed_ungated_transfer`.
    public fun ungated_transfer_allowed(wallet: &Object<FungibleAssetWallet>): bool acquires FungibleAssetWallet {
        borrow_fungible_asset(wallet).allow_ungated_transfer
    }

    /// Mint the `amount` of fungible asset.
    public fun mint(
        ref: &MintRef,
        amount: u64,
    ): FungibleAsset acquires FungibleAssetMetadata {
        assert!(amount != 0, error::invalid_argument(EAMOUNT_CANNOT_BE_ZERO));
        let metadata = verify(&ref.metadata);
        increase_supply(&ref.metadata, amount);
        FungibleAsset {
            metadata,
            amount
        }
    }

    /// Enable/disable the direct transfer of fungible asset.
    public fun set_ungated_transfer(
        ref: &TransferRef,
        wallet: &Object<FungibleAssetWallet>,
        allow: bool,
    ) acquires FungibleAssetWallet {
        assert!(
            &ref.metadata == &metadata_from_wallet(wallet),
            error::invalid_argument(ETRANSFER_REF_AND_WALLET_MISMATCH)
        );
        borrow_fungible_asset_mut(wallet).allow_ungated_transfer = allow;
    }

    /// Burn the `amount` of fungible metadata from `account`.
    public fun burn(
        ref: &BurnRef,
        wallet: &Object<FungibleAssetWallet>,
        amount: u64
    ) acquires FungibleAssetWallet, FungibleAssetMetadata {
        assert!(
            &ref.metadata == &metadata_from_wallet(wallet),
            error::invalid_argument(EBURN_REF_AND_WALLET_MISMATCH)
        );
        let FungibleAsset {
            metadata,
            amount,
        } = extract(wallet, amount);
        decrease_supply(&metadata, amount);
    }

    /// Withdarw `amount` of fungible asset from `wallet` by the owner.
    public fun withdraw(
        account: &signer,
        wallet: &Object<FungibleAssetWallet>,
        amount: u64
    ): FungibleAsset acquires FungibleAssetWallet {
        assert_owner(account, wallet);
        extract(wallet, amount)
    }

    /// Deposit `amount` of fungible asset to `wallet`.
    public fun deposit(
        wallet: &Object<FungibleAssetWallet>,
        fa: FungibleAsset,
    ) acquires FungibleAssetWallet {
        let FungibleAsset { metadata, amount } = fa;
        // ensure merging the same coin
        let wallet = borrow_fungible_asset_mut(wallet);
        assert!(wallet.allow_ungated_transfer, error::invalid_argument(EUNGATED_TRANSFER_IS_NOT_ALLOWED));
        assert!(wallet.metadata == metadata, error::invalid_argument(EFUNGIBLE_ASSET_AND_WALLET_MISMATCH));
        wallet.balance = wallet.balance + amount;
    }

    /// Transfer `amount` of fungible metadata of `metadata` to `receiver`.
    /// Note: it does not move the underlying object.
    public fun transfer(
        account: &signer,
        amount: u64,
        from: &Object<FungibleAssetWallet>,
        to: &Object<FungibleAssetWallet>,
    ) acquires FungibleAssetWallet {
        let fa = withdraw(account, from, amount);
        deposit(to, fa);
    }

    /// Withdarw `amount` of fungible metadata from `account` ignoring `allow_ungated_transfer`.
    public fun withdraw_with_ref(
        ref: &TransferRef,
        wallet: &Object<FungibleAssetWallet>,
        amount: u64
    ): FungibleAsset acquires FungibleAssetWallet {
        assert!(
            &ref.metadata == &metadata_from_wallet(wallet),
            error::invalid_argument(ETRANSFER_REF_AND_WALLET_MISMATCH)
        );
        let ungated_transfer_allowed = ungated_transfer_allowed(wallet);
        if (!ungated_transfer_allowed) {
            set_ungated_transfer(ref, wallet, true);
        };
        let fa = extract(wallet, amount);
        if (!ungated_transfer_allowed) {
            set_ungated_transfer(ref, wallet, false);
        };
        fa
    }

    /// Deposit fungible asset into `account` ignoring `allow_ungated_transfer`.
    public fun deposit_with_ref(
        ref: &TransferRef,
        wallet: &Object<FungibleAssetWallet>,
        fa: FungibleAsset
    ) acquires FungibleAssetWallet {
        assert!(
            &ref.metadata == &metadata_from_wallet(wallet),
            error::invalid_argument(ETRANSFER_REF_AND_WALLET_MISMATCH)
        );
        let ungated_transfer_allowed = ungated_transfer_allowed(wallet);
        if (!ungated_transfer_allowed) {
            set_ungated_transfer(ref, wallet, true);
        };
        deposit(wallet, fa);
        if (!ungated_transfer_allowed) {
            set_ungated_transfer(ref, wallet, false);
        };
    }

    /// Transfer `ammount` of  fungible metadata with `TransferRef` even ungated transfer is disabled.
    public fun transfer_with_ref(
        transfer_ref: &TransferRef,
        from: &Object<FungibleAssetWallet>,
        to: &Object<FungibleAssetWallet>,
        amount: u64,
    ) acquires FungibleAssetWallet {
        let fa = withdraw_with_ref(transfer_ref, from, amount);
        deposit_with_ref(transfer_ref, to, fa);
    }

    /// Get the underlying metadata object from `MintRef`.
    public fun mint_ref_metadata(ref: &MintRef): Object<FungibleAssetMetadata> {
        ref.metadata
    }

    /// Get the underlying metadata object from `TransferRef`.
    public fun transfer_ref_metadata(ref: &TransferRef): Object<FungibleAssetMetadata> {
        ref.metadata
    }

    /// Get the underlying metadata object from `BurnRef`.
    public fun burn_ref_metadata(ref: &BurnRef): Object<FungibleAssetMetadata> {
        ref.metadata
    }

    /// Assert the owner of `metadata`.
    inline fun assert_owner<T: key>(owner: &signer, metadata: &Object<T>) {
        assert!(object::is_owner(*metadata, signer::address_of(owner)), error::permission_denied(ENOT_OWNER));
    }

    /// Extract `amount` of fungible asset from `wallet`.
    fun extract(
        wallet: &Object<FungibleAssetWallet>,
        amount: u64,
    ): FungibleAsset acquires FungibleAssetWallet {
        assert!(amount != 0, error::invalid_argument(EAMOUNT_CANNOT_BE_ZERO));
        let wallet = borrow_fungible_asset_mut(wallet);
        assert!(wallet.allow_ungated_transfer, error::invalid_argument(EUNGATED_TRANSFER_IS_NOT_ALLOWED));
        assert!(wallet.balance >= amount, error::invalid_argument(EINSUFFICIENT_BALANCE));
        wallet.balance = wallet.balance - amount;
        FungibleAsset {
            metadata: wallet.metadata,
            amount
        }
    }

    /// Increase the supply of a fungible metadata by minting.
    fun increase_supply<T: key>(metadata: &Object<T>, amount: u64) acquires FungibleAssetMetadata {
        assert!(amount != 0, error::invalid_argument(EZERO_AMOUNT));
        let fungible_metadata = borrow_fungible_metadata_mut(metadata);
        if (option::is_some(&fungible_metadata.maximum)) {
            let max = *option::borrow(&fungible_metadata.maximum);
            assert!(max - fungible_metadata.supply >= amount, error::invalid_argument(ECURRENT_SUPPLY_OVERFLOW))
        };
        fungible_metadata.supply = fungible_metadata.supply + amount;
    }

    /// Decrease the supply of a fungible metadata by burning.
    fun decrease_supply<T: key>(metadata: &Object<T>, amount: u64) acquires FungibleAssetMetadata {
        assert!(amount != 0, error::invalid_argument(EZERO_AMOUNT));
        let fungible_metadata = borrow_fungible_metadata_mut(metadata);
        assert!(fungible_metadata.supply >= amount, error::invalid_argument(ECURRENT_SUPPLY_UNDERFLOW));
        fungible_metadata.supply = fungible_metadata.supply - amount;
    }

    inline fun borrow_fungible_metadata<T: key>(
        metadata: &Object<T>
    ): &FungibleAssetMetadata acquires FungibleAssetMetadata {
        let addr = object::object_address(&verify(metadata));
        borrow_global<FungibleAssetMetadata>(addr)
    }

    inline fun borrow_fungible_metadata_mut<T: key>(
        metadata: &Object<T>
    ): &mut FungibleAssetMetadata acquires FungibleAssetMetadata {
        let addr = object::object_address(&verify(metadata));
        borrow_global_mut<FungibleAssetMetadata>(addr)
    }

    inline fun borrow_fungible_asset(
        wallet: &Object<FungibleAssetWallet>,
    ): &FungibleAssetWallet acquires FungibleAssetWallet {
        borrow_global<FungibleAssetWallet>(object::object_address(wallet))
    }

    inline fun borrow_fungible_asset_mut(
        wallet: &Object<FungibleAssetWallet>,
    ): &mut FungibleAssetWallet acquires FungibleAssetWallet {
        borrow_global_mut<FungibleAssetWallet>(object::object_address(wallet))
    }

    #[test_only]
    use std::string;
    #[test_only]
    use aptos_framework::account;
    #[test_only]
    use aptos_framework::object::object_address;

    #[test_only]
    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct TestToken has key {}

    #[test_only]
    public fun create_test_token(creator: &signer): (ConstructorRef, Object<TestToken>) {
        account::create_account_for_test(signer::address_of(creator));
        let creator_ref = object::create_object_from_account(creator);
        let object_signer = object::generate_signer(&creator_ref);
        move_to(&object_signer, TestToken {});

        let token = object::object_from_constructor_ref<TestToken>(&creator_ref);
        (creator_ref, token)
    }

    #[test_only]
    public fun init_test_metadata(creator_ref: &ConstructorRef): (MintRef, TransferRef, BurnRef) {
        init_metadata(
            creator_ref,
            100 /* max supply */,
            string::utf8(b"USDA"),
            string::utf8(b"$$$"),
            0
        )
    }

    #[test_only]
    public fun generate_refs(
        creator: &signer
    ): (MintRef, TransferRef, BurnRef, Object<TestToken>) {
        let (creator_ref, metadata) = create_test_token(creator);
        let (mint, transfer, burn) = init_test_metadata(&creator_ref);
        (mint, transfer, burn, metadata)
    }

    #[test_only]
    fun generate_wallet(
        creator: &signer,
        metadata: &Object<FungibleAssetMetadata>,
    ): Object<FungibleAssetWallet> {
        if (!account::exists_at(signer::address_of(creator))) {
            account::create_account_for_test(signer::address_of(creator));
        };
        let wallet_creator_ref = object::create_object_from_account(creator);
        new_fungible_asset_wallet_object(&wallet_creator_ref, metadata)
    }

    #[test(creator = @0xcafe)]
    fun test_metadata_basic_flow(creator: &signer) acquires FungibleAssetMetadata {
        let (creator_ref, asset) = create_test_token(creator);
        init_test_metadata(&creator_ref);
        assert!(supply(&asset) == 0, 1);
        assert!(maximum(&asset) == option::some(100), 2);
        assert!(name(&asset) == string::utf8(b"USDA"), 3);
        assert!(symbol(&asset) == string::utf8(b"$$$"), 4);
        assert!(decimals(&asset) == 0, 5);

        increase_supply(&asset, 50);
        assert!(supply(&asset) == 50, 6);
        decrease_supply(&asset, 30);
        assert!(supply(&asset) == 20, 7);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 0x10008, location = Self)]
    fun test_supply_overflow(creator: &signer) acquires FungibleAssetMetadata {
        let (creator_ref, asset) = create_test_token(creator);
        init_test_metadata(&creator_ref);
        increase_supply(&asset, 101);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 0x10009, location = Self)]
    fun test_supply_underflow(creator: &signer) acquires FungibleAssetMetadata {
        let (creator_ref, asset) = create_test_token(creator);
        init_test_metadata(&creator_ref);
        decrease_supply(&asset, 1);
    }

    #[test(creator = @0xcafe, aaron = @0xface)]
    fun test_e2e_basic_flow(
        creator: &signer,
        aaron: &signer,
    ) acquires FungibleAssetWallet, FungibleAssetMetadata {
        let (mint_ref, transfer_ref, burn_ref, metadata) = generate_refs(creator);
        let metadata = verify(&metadata);
        let creator_wallet = generate_wallet(creator, &metadata);
        let aaron_wallet = generate_wallet(aaron, &metadata);

        assert!(supply(&metadata) == 0, 1);
        // Mint
        let fa = mint(&mint_ref, 100);
        assert!(supply(&metadata) == 100, 2);
        // Deposit
        deposit(&creator_wallet, fa);
        // Withdraw
        let fa = withdraw(creator, &creator_wallet, 80);
        assert!(supply(&metadata) == 100, 3);
        deposit(&aaron_wallet, fa);
        // Burn
        burn(&burn_ref, &aaron_wallet, 30);
        assert!(supply(&metadata) == 70, 4);
        // Transfer
        transfer(creator, 10, &creator_wallet, &aaron_wallet);
        assert!(balance(&creator_wallet) == 10, 5);
        assert!(balance(&aaron_wallet) == 60, 6);

        set_ungated_transfer(&transfer_ref, &aaron_wallet, false);
        assert!(!ungated_transfer_allowed(&aaron_wallet), 7);

        destory_fungible_asset_wallet(aaron_wallet);
        assert!(!exists<FungibleAssetWallet>(object_address(&aaron_wallet)), 8);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 0x10004, location = Self)]
    fun test_ungated_transfer(creator: &signer) acquires FungibleAssetMetadata, FungibleAssetWallet {
        let (mint_ref, transfer_ref, _burn_ref, metadata) = generate_refs(creator);
        let metadata = verify(&metadata);
        let creator_wallet = generate_wallet(creator, &metadata);

        let fa = mint(&mint_ref, 100);
        set_ungated_transfer(&transfer_ref, &creator_wallet, false);
        deposit(&creator_wallet, fa);
    }

    #[test(creator = @0xcafe, aaron = @0xface)]
    fun test_transfer_with_ref(creator: &signer, aaron: &signer) acquires FungibleAssetMetadata, FungibleAssetWallet {
        let (mint_ref, transfer_ref, _burn_ref, metadata) = generate_refs(creator);
        let metadata = verify(&metadata);
        let creator_wallet = generate_wallet(creator, &metadata);
        let aaron_wallet = generate_wallet(aaron, &metadata);

        let fa = mint(&mint_ref, 100);
        set_ungated_transfer(&transfer_ref, &creator_wallet, false);
        set_ungated_transfer(&transfer_ref, &aaron_wallet, false);
        deposit_with_ref(&transfer_ref, &creator_wallet, fa);
        transfer_with_ref(&transfer_ref, &creator_wallet, &aaron_wallet, 80);
        assert!(balance(&creator_wallet) == 20, 1);
        assert!(balance(&aaron_wallet) == 80, 2);
        assert!(!ungated_transfer_allowed(&creator_wallet), 3);
        assert!(!ungated_transfer_allowed(&aaron_wallet), 4);
    }
}
