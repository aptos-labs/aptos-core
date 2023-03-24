/// This defines the fungible asset module that can issue fungible asset of any `FungibleAssetMetadata` object. The
/// metadata object can be any object that equipped with `FungibleAssetMetadata` resource.
module aptos_framework::fungible_asset {
    use aptos_framework::create_signer;
    use aptos_framework::object::{Self, Object, ConstructorRef, DeriveRef};

    use std::error;
    use std::option::{Self, Option};
    use std::signer;
    use std::string::String;

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
    /// The signer is not the owner of the wallet.
    const ENOT_WALLET_OWNER: u64 = 10;
    /// The mint ref and the the wallet do not match.
    const EMINT_REF_AND_WALLET_MISMATCH: u64 = 11;

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
        /// The ref used to create wallet objects for users later.
        derive_ref: DeriveRef,
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
                derive_ref: object::generate_derive_ref(constructor_ref),
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

    /// Create a new wallet object to hold fungible asset.
    public fun create_deterministic_wallet<T: key>(
        owner_addr: address,
        metadata: Object<T>,
    ): Object<FungibleAssetWallet> acquires FungibleAssetMetadata {
        let owner = &create_signer::create_signer(owner_addr);
        let derive_ref = &borrow_fungible_metadata(&metadata).derive_ref;
        let constructor_ref = &object::create_derived_object(owner, derive_ref);
        initialize_arbitrary_wallet(constructor_ref, metadata)
    }

    public fun deterministic_wallet_address<T: key>(owner: address, metadata: Object<T>): address {
        let metadata_addr = object::object_address(&metadata);
        object::create_derived_object_address(owner, metadata_addr)
    }

    /// Allow an object to hold a wallet for fungible assets.
    /// Applications can use this to create multiple wallets for isolating fungible assets for different purposes.
    public fun initialize_arbitrary_wallet<T: key>(
        constructor_ref: &ConstructorRef,
        metadata: Object<T>,
    ): Object<FungibleAssetWallet> {
        let wallet_obj = &object::generate_signer(constructor_ref);
        let metadata = object::convert<T, FungibleAssetMetadata>(metadata);
        move_to(wallet_obj, FungibleAssetWallet {
            metadata,
            balance: 0,
            allow_ungated_transfer: true,
        });

        object::object_from_constructor_ref<FungibleAssetWallet>(constructor_ref)
    }

    /// Return the underlying metadata object
    public fun metadata_from_asset(fa: &FungibleAsset): Object<FungibleAssetMetadata> {
        fa.metadata
    }

    /// Return the underlying metadata object.
    public fun metadata_from_wallet<T: key>(wallet: &Object<T>): Object<FungibleAssetMetadata> acquires FungibleAssetWallet {
        borrow_wallet_resource(wallet).metadata
    }

    /// Return `amount` inside.
    public fun amount(fa: &FungibleAsset): u64 {
        fa.amount
    }

    /// Return whether the provided address has a wallet initialized.
    public fun wallet_exists(wallet: address): bool {
        exists<FungibleAssetWallet>(wallet)
    }

    /// Get the balance of a given wallet.
    public fun balance<T: key>(wallet: Object<T>): u64 acquires FungibleAssetWallet {
        if (wallet_exists(object::object_address(&wallet))) {
            borrow_wallet_resource(&wallet).balance
        } else {
            0
        }
    }

    /// Return whether a wallet can freely send or receive fungible assets.
    public fun ungated_transfer_allowed<T: key>(wallet: Object<T>): bool acquires FungibleAssetWallet {
        borrow_wallet_resource(&wallet).allow_ungated_transfer
    }

    /// Mint the `amount` of fungible asset.
    public fun mint(ref: &MintRef, amount: u64): FungibleAsset acquires FungibleAssetMetadata {
        assert!(amount > 0, error::invalid_argument(EAMOUNT_CANNOT_BE_ZERO));
        let metadata = ref.metadata;
        increase_supply(&metadata, amount);
        FungibleAsset {
            metadata,
            amount
        }
    }

    public fun mint_to<T: key>(
        ref: &MintRef,
        wallet: Object<T>,
        amount: u64,
    ) acquires FungibleAssetMetadata, FungibleAssetWallet {
        deposit(wallet, mint(ref, amount));
    }

    /// Enable/disable a wallet's ability to do direct transfers of fungible asset.
    public fun set_ungated_transfer<T: key>(
        ref: &TransferRef,
        wallet: Object<T>,
        allow: bool,
    ) acquires FungibleAssetWallet {
        assert!(
            ref.metadata == metadata_from_wallet(&wallet),
            error::invalid_argument(ETRANSFER_REF_AND_WALLET_MISMATCH),
        );
        let wallet_addr = object::object_address(&wallet);
        borrow_global_mut<FungibleAssetWallet>(wallet_addr).allow_ungated_transfer = allow;
    }

    /// Burn the `amount` of fungible metadata from the given wallet.
    public fun burn<T: key>(
        ref: &BurnRef,
        wallet: Object<T>,
        amount: u64
    ) acquires FungibleAssetWallet, FungibleAssetMetadata {
        let metadata = ref.metadata;
        assert!(metadata == metadata_from_wallet(&wallet), error::invalid_argument(EBURN_REF_AND_WALLET_MISMATCH));
        let wallet_addr = object::object_address(&wallet);
        let FungibleAsset {
            metadata,
            amount,
        } = extract(wallet_addr, amount);
        decrease_supply(&metadata, amount);
    }

    /// Withdraw `amount` of fungible asset from `wallet` by the owner.
    public fun withdraw<T: key>(
        owner: &signer,
        wallet: Object<T>,
        amount: u64,
    ): FungibleAsset acquires FungibleAssetWallet {
        assert!(object::owns(wallet, signer::address_of(owner)), error::permission_denied(ENOT_WALLET_OWNER));
        assert!(ungated_transfer_allowed(wallet), error::invalid_argument(EUNGATED_TRANSFER_IS_NOT_ALLOWED));
        extract(object::object_address(&wallet), amount)
    }

    /// Deposit `amount` of fungible asset to `wallet`.
    public fun deposit<T: key>(wallet: Object<T>, fa: FungibleAsset) acquires FungibleAssetWallet {
        assert!(ungated_transfer_allowed(wallet), error::invalid_argument(EUNGATED_TRANSFER_IS_NOT_ALLOWED));
        deposit_internal(wallet, fa);
    }

    /// Transfer `amount` of fungible asset from `from_wallet` which should be owned by `sender` to `to_wallet`.
    /// Note: it does not move the underlying object.
    public fun transfer<T: key>(
        sender: &signer,
        from_wallet: Object<T>,
        amount: u64,
        to_wallet: Object<T>,
    ) acquires FungibleAssetWallet {
        let fa = withdraw(sender, from_wallet, amount);
        deposit(to_wallet, fa);
    }

    /// Withdraw `amount` of fungible metadata from `wallet` ignoring `allow_ungated_transfer`.
    public fun withdraw_with_ref<T: key>(
        ref: &TransferRef,
        wallet: Object<T>,
        amount: u64
    ): FungibleAsset acquires FungibleAssetWallet {
        assert!(
            ref.metadata == metadata_from_wallet(&wallet),
            error::invalid_argument(ETRANSFER_REF_AND_WALLET_MISMATCH),
        );
        extract(object::object_address(&wallet), amount)
    }

    /// Deposit fungible asset into `wallet` ignoring `allow_ungated_transfer`.
    public fun deposit_with_ref<T: key>(
        ref: &TransferRef,
        wallet: Object<T>,
        fa: FungibleAsset
    ) acquires FungibleAssetWallet {
        assert!(
            ref.metadata == fa.metadata,
            error::invalid_argument(ETRANSFER_REF_AND_WALLET_MISMATCH)
        );
        deposit_internal(wallet, fa);
    }

    /// Transfer `ammount` of  fungible metadata with `TransferRef` even ungated transfer is disabled.
    public fun transfer_with_ref<T: key>(
        transfer_ref: &TransferRef,
        from_wallet: Object<T>,
        amount: u64,
        to_wallet: Object<T>,
    ) acquires FungibleAssetWallet {
        let fa = withdraw_with_ref(transfer_ref, from_wallet, amount);
        deposit_with_ref(transfer_ref, to_wallet, fa);
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

    public fun asset_metadata(fa: &FungibleAsset): Object<FungibleAssetMetadata> {
        fa.metadata
    }

    fun deposit_internal<T: key>(wallet: Object<T>, fa: FungibleAsset) acquires FungibleAssetWallet {
        let FungibleAsset { metadata, amount } = fa;
        let wallet_metadata = metadata_from_wallet(&wallet);
        assert!(metadata == wallet_metadata, error::invalid_argument(EFUNGIBLE_ASSET_AND_WALLET_MISMATCH));
        let wallet_addr = object::object_address(&wallet);
        let wallet = borrow_global_mut<FungibleAssetWallet>(wallet_addr);
        wallet.balance = wallet.balance + amount;
    }

    /// Extract `amount` of fungible asset from `wallet`.
    fun extract(wallet_addr: address, amount: u64): FungibleAsset acquires FungibleAssetWallet {
        assert!(amount != 0, error::invalid_argument(EAMOUNT_CANNOT_BE_ZERO));
        let wallet = borrow_global_mut<FungibleAssetWallet>(wallet_addr);
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
        let addr = object::object_address(metadata);
        borrow_global<FungibleAssetMetadata>(addr)
    }

    inline fun borrow_fungible_metadata_mut<T: key>(
        metadata: &Object<T>
    ): &mut FungibleAssetMetadata acquires FungibleAssetMetadata {
        let addr = object::object_address(metadata);
        borrow_global_mut<FungibleAssetMetadata>(addr)
    }

    inline fun borrow_wallet_resource<T: key>(wallet: &Object<T>): &FungibleAssetWallet acquires FungibleAssetWallet {
        borrow_global<FungibleAssetWallet>(object::object_address(wallet))
    }

    #[test_only]
    use std::string;
    #[test_only]
    use aptos_framework::account;

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
    public fun create_fungible_asset(
        creator: &signer
    ): (MintRef, TransferRef, BurnRef, Object<TestToken>) {
        let (creator_ref, metadata) = create_test_token(creator);
        let (mint, transfer, burn) = init_test_metadata(&creator_ref);
        (mint, transfer, burn, metadata)
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
        let (mint_ref, transfer_ref, burn_ref, test_token) = create_fungible_asset(creator);
        let metadata = mint_ref.metadata;
        let creator_wallet = create_deterministic_wallet(signer::address_of(creator), metadata);
        let aaron_wallet = create_deterministic_wallet(signer::address_of(aaron), metadata);

        assert!(supply(&test_token) == 0, 1);
        // Mint
        let fa = mint(&mint_ref, 100);
        assert!(supply(&test_token) == 100, 2);
        // Deposit
        deposit(creator_wallet, fa);
        // Withdraw
        let fa = withdraw(creator, creator_wallet, 80);
        assert!(supply(&test_token) == 100, 3);
        deposit(aaron_wallet, fa);
        // Burn
        burn(&burn_ref, aaron_wallet, 30);
        assert!(supply(&test_token) == 70, 4);
        // Transfer
        transfer(creator, creator_wallet,10, aaron_wallet);
        assert!(balance(creator_wallet) == 10, 5);
        assert!(balance(aaron_wallet) == 60, 6);

        set_ungated_transfer(&transfer_ref, aaron_wallet, false);
        assert!(!ungated_transfer_allowed(aaron_wallet), 7);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 0x10004, location = Self)]
    fun test_ungated_transfer(creator: &signer) acquires FungibleAssetMetadata, FungibleAssetWallet {
        let (mint_ref, transfer_ref, _burn_ref, _) = create_fungible_asset(creator);

        let creator_wallet = create_deterministic_wallet(signer::address_of(creator), mint_ref.metadata);
        let fa = mint(&mint_ref, 100);
        set_ungated_transfer(&transfer_ref, creator_wallet, false);
        deposit(creator_wallet, fa);
    }

    #[test(creator = @0xcafe, aaron = @0xface)]
    fun test_transfer_with_ref(creator: &signer, aaron: &signer) acquires FungibleAssetMetadata, FungibleAssetWallet {
        let (mint_ref, transfer_ref, _burn_ref, _) = create_fungible_asset(creator);
        let metadata = mint_ref.metadata;
        let creator_wallet = create_deterministic_wallet(signer::address_of(creator), metadata);
        let aaron_wallet = create_deterministic_wallet(signer::address_of(aaron), metadata);

        let fa = mint(&mint_ref, 100);
        set_ungated_transfer(&transfer_ref, creator_wallet, false);
        set_ungated_transfer(&transfer_ref, aaron_wallet, false);
        deposit_with_ref(&transfer_ref, creator_wallet, fa);
        transfer_with_ref(&transfer_ref, creator_wallet, 80, aaron_wallet);
        assert!(balance(creator_wallet) == 20, 1);
        assert!(balance(aaron_wallet) == 80, 2);
        assert!(!ungated_transfer_allowed(creator_wallet), 3);
        assert!(!ungated_transfer_allowed(aaron_wallet), 4);
    }
}
