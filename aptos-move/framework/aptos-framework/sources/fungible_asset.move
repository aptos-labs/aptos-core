/// This defines the fungible asset module that can issue fungible asset of any `Metadata` object. The
/// metadata object can be any object that equipped with `Metadata` resource.
module aptos_framework::fungible_asset {
    use aptos_framework::aggregator_v2::{Self, Aggregator};
    use aptos_framework::event;
    use aptos_framework::object::{Self, Object, ConstructorRef, DeleteRef, ExtendRef};
    use std::string;
    use std::features;

    use std::error;
    use std::option::{Self, Option};
    use std::signer;
    use std::string::String;

    /// Amount cannot be zero.
    const EAMOUNT_CANNOT_BE_ZERO: u64 = 1;
    /// The transfer ref and the fungible asset do not match.
    const ETRANSFER_REF_AND_FUNGIBLE_ASSET_MISMATCH: u64 = 2;
    /// Store is disabled from sending and receiving this fungible asset.
    const ESTORE_IS_FROZEN: u64 = 3;
    /// Insufficient balance to withdraw or transfer.
    const EINSUFFICIENT_BALANCE: u64 = 4;
    /// The fungible asset's supply has exceeded maximum.
    const EMAX_SUPPLY_EXCEEDED: u64 = 5;
    /// Fungible asset do not match when merging.
    const EFUNGIBLE_ASSET_MISMATCH: u64 = 6;
    /// The mint ref and the the store do not match.
    const EMINT_REF_AND_STORE_MISMATCH: u64 = 7;
    /// Account is not the store's owner.
    const ENOT_STORE_OWNER: u64 = 8;
    /// Transfer ref and store do not match.
    const ETRANSFER_REF_AND_STORE_MISMATCH: u64 = 9;
    /// Burn ref and store do not match.
    const EBURN_REF_AND_STORE_MISMATCH: u64 = 10;
    /// Fungible asset and store do not match.
    const EFUNGIBLE_ASSET_AND_STORE_MISMATCH: u64 = 11;
    /// Cannot destroy non-empty fungible assets.
    const EAMOUNT_IS_NOT_ZERO: u64 = 12;
    /// Burn ref and fungible asset do not match.
    const EBURN_REF_AND_FUNGIBLE_ASSET_MISMATCH: u64 = 13;
    /// Cannot destroy fungible stores with a non-zero balance.
    const EBALANCE_IS_NOT_ZERO: u64 = 14;
    /// Name of the fungible asset metadata is too long
    const ENAME_TOO_LONG: u64 = 15;
    /// Symbol of the fungible asset metadata is too long
    const ESYMBOL_TOO_LONG: u64 = 16;
    /// Decimals is over the maximum of 32
    const EDECIMALS_TOO_LARGE: u64 = 17;
    /// Fungibility is only available for non-deletable objects.
    const EOBJECT_IS_DELETABLE: u64 = 18;
    /// URI for the icon of the fungible asset metadata is too long
    const EURI_TOO_LONG: u64 = 19;
    /// The fungible asset's supply will be negative which should be impossible.
    const ESUPPLY_UNDERFLOW: u64 = 20;
    /// Supply resource is not found for a metadata object.
    const ESUPPLY_NOT_FOUND: u64 = 21;
    /// Flag for Concurrent Supply not enabled
    const ECONCURRENT_SUPPLY_NOT_ENABLED: u64 = 22;

    //
    // Constants
    //

    const MAX_NAME_LENGTH: u64 = 32;
    const MAX_SYMBOL_LENGTH: u64 = 10;
    const MAX_DECIMALS: u8 = 32;
    const MAX_URI_LENGTH: u64 = 512;

    /// Maximum possible coin supply.
    const MAX_U128: u128 = 340282366920938463463374607431768211455;

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct Supply has key {
        current: u128,
        // option::none() means unlimited supply.
        maximum: Option<u128>,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct ConcurrentSupply has key {
        current: Aggregator<u128>,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Metadata of a Fungible asset
    struct Metadata has key {
        /// Name of the fungible metadata, i.e., "USDT".
        name: String,
        /// Symbol of the fungible metadata, usually a shorter version of the name.
        /// For example, Singapore Dollar is SGD.
        symbol: String,
        /// Number of decimals used for display purposes.
        /// For example, if `decimals` equals `2`, a balance of `505` coins should
        /// be displayed to a user as `5.05` (`505 / 10 ** 2`).
        decimals: u8,
        /// The Uniform Resource Identifier (uri) pointing to an image that can be used as the icon for this fungible
        /// asset.
        icon_uri: String,
        /// The Uniform Resource Identifier (uri) pointing to the website for the fungible asset.
        project_uri: String,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// The store object that holds fungible assets of a specific type associated with an account.
    struct FungibleStore has key {
        /// The address of the base metadata object.
        metadata: Object<Metadata>,
        /// The balance of the fungible metadata.
        balance: u64,
        /// If true, owner transfer is disabled that only `TransferRef` can move in/out from this store.
        frozen: bool,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct FungibleAssetEvents has key {
        deposit_events: event::EventHandle<DepositEvent>,
        withdraw_events: event::EventHandle<WithdrawEvent>,
        frozen_events: event::EventHandle<FrozenEvent>,
    }

    /// FungibleAsset can be passed into function for type safety and to guarantee a specific amount.
    /// FungibleAsset is ephemeral and cannot be stored directly. It must be deposited back into a store.
    struct FungibleAsset {
        metadata: Object<Metadata>,
        amount: u64,
    }

    /// MintRef can be used to mint the fungible asset into an account's store.
    struct MintRef has drop, store {
        metadata: Object<Metadata>
    }

    /// TransferRef can be used to allow or disallow the owner of fungible assets from transferring the asset
    /// and allow the holder of TransferRef to transfer fungible assets from any account.
    struct TransferRef has drop, store {
        metadata: Object<Metadata>
    }

    /// BurnRef can be used to burn fungible assets from a given holder account.
    struct BurnRef has drop, store {
        metadata: Object<Metadata>
    }

    /// Emitted when fungible assets are deposited into a store.
    struct DepositEvent has drop, store {
        amount: u64,
    }

    /// Emitted when fungible assets are withdrawn from a store.
    struct WithdrawEvent has drop, store {
        amount: u64,
    }

    /// Emitted when a store's frozen status is updated.
    struct FrozenEvent has drop, store {
        frozen: bool,
    }

    /// Make an existing object fungible by adding the Metadata resource.
    /// This returns the capabilities to mint, burn, and transfer.
    /// maximum_supply defines the behavior of maximum supply when monitoring:
    ///   - option::none(): Monitoring unlimited supply
    ///     (width of the field - MAX_U128 is the implicit maximum supply)
    ///     if option::some(MAX_U128) is used, it is treated as unlimited supply.
    ///   - option::some(max): Monitoring fixed supply with `max` as the maximum supply.
    public fun add_fungibility(
        constructor_ref: &ConstructorRef,
        maximum_supply: Option<u128>,
        name: String,
        symbol: String,
        decimals: u8,
        icon_uri: String,
        project_uri: String,
    ): Object<Metadata> {
        assert!(!object::can_generate_delete_ref(constructor_ref), error::invalid_argument(EOBJECT_IS_DELETABLE));
        let metadata_object_signer = &object::generate_signer(constructor_ref);
        assert!(string::length(&name) <= MAX_NAME_LENGTH, error::out_of_range(ENAME_TOO_LONG));
        assert!(string::length(&symbol) <= MAX_SYMBOL_LENGTH, error::out_of_range(ESYMBOL_TOO_LONG));
        assert!(decimals <= MAX_DECIMALS, error::out_of_range(EDECIMALS_TOO_LARGE));
        assert!(string::length(&icon_uri) <= MAX_URI_LENGTH, error::out_of_range(EURI_TOO_LONG));
        assert!(string::length(&project_uri) <= MAX_URI_LENGTH, error::out_of_range(EURI_TOO_LONG));
        move_to(metadata_object_signer,
            Metadata {
                name,
                symbol,
                decimals,
                icon_uri,
                project_uri,
            }
        );

        if (features::concurrent_assets_enabled()) {
            let unlimited = option::is_none(&maximum_supply);
            move_to(metadata_object_signer, ConcurrentSupply {
                current: if (unlimited) {
                    aggregator_v2::create_unbounded_aggregator()
                } else {
                    aggregator_v2::create_aggregator(option::extract(&mut maximum_supply))
                },
            });
        } else {
            move_to(metadata_object_signer, Supply {
                current: 0,
                maximum: maximum_supply
            });
        };

        object::object_from_constructor_ref<Metadata>(constructor_ref)
    }

    /// Creates a mint ref that can be used to mint fungible assets from the given fungible object's constructor ref.
    /// This can only be called at object creation time as constructor_ref is only available then.
    public fun generate_mint_ref(constructor_ref: &ConstructorRef): MintRef {
        let metadata = object::object_from_constructor_ref<Metadata>(constructor_ref);
        MintRef { metadata }
    }

    /// Creates a burn ref that can be used to burn fungible assets from the given fungible object's constructor ref.
    /// This can only be called at object creation time as constructor_ref is only available then.
    public fun generate_burn_ref(constructor_ref: &ConstructorRef): BurnRef {
        let metadata = object::object_from_constructor_ref<Metadata>(constructor_ref);
        BurnRef { metadata }
    }

    /// Creates a transfer ref that can be used to freeze/unfreeze/transfer fungible assets from the given fungible
    /// object's constructor ref.
    /// This can only be called at object creation time as constructor_ref is only available then.
    public fun generate_transfer_ref(constructor_ref: &ConstructorRef): TransferRef {
        let metadata = object::object_from_constructor_ref<Metadata>(constructor_ref);
        TransferRef { metadata }
    }

    #[view]
    /// Get the current supply from the `metadata` object.
    public fun supply<T: key>(metadata: Object<T>): Option<u128> acquires Supply, ConcurrentSupply {
        let metadata_address = object::object_address(&metadata);
        if (exists<ConcurrentSupply>(metadata_address)) {
            let supply = borrow_global<ConcurrentSupply>(metadata_address);
            option::some(aggregator_v2::read(&supply.current))
        } else if (exists<Supply>(metadata_address)) {
            let supply = borrow_global<Supply>(metadata_address);
            option::some(supply.current)
        } else {
            option::none()
        }
    }

    #[view]
    /// Get the maximum supply from the `metadata` object.
    /// If supply is unlimited (or set explicitly to MAX_U128), none is returned
    public fun maximum<T: key>(metadata: Object<T>): Option<u128> acquires Supply, ConcurrentSupply {
        let metadata_address = object::object_address(&metadata);
        if (exists<ConcurrentSupply>(metadata_address)) {
            let supply = borrow_global<ConcurrentSupply>(metadata_address);
            let max_value = aggregator_v2::max_value(&supply.current);
            if (max_value == MAX_U128) {
                option::none()
            } else {
                option::some(max_value)
            }
        } else if (exists<Supply>(metadata_address)) {
            let supply = borrow_global<Supply>(metadata_address);
            supply.maximum
        } else {
            option::none()
        }
    }

    #[view]
    /// Get the name of the fungible asset from the `metadata` object.
    public fun name<T: key>(metadata: Object<T>): String acquires Metadata {
        borrow_fungible_metadata(&metadata).name
    }

    #[view]
    /// Get the symbol of the fungible asset from the `metadata` object.
    public fun symbol<T: key>(metadata: Object<T>): String acquires Metadata {
        borrow_fungible_metadata(&metadata).symbol
    }

    #[view]
    /// Get the decimals from the `metadata` object.
    public fun decimals<T: key>(metadata: Object<T>): u8 acquires Metadata {
        borrow_fungible_metadata(&metadata).decimals
    }

    #[view]
    /// Return whether the provided address has a store initialized.
    public fun store_exists(store: address): bool {
        exists<FungibleStore>(store)
    }

    /// Return the underlying metadata object
    public fun metadata_from_asset(fa: &FungibleAsset): Object<Metadata> {
        fa.metadata
    }

    #[view]
    /// Return the underlying metadata object.
    public fun store_metadata<T: key>(store: Object<T>): Object<Metadata> acquires FungibleStore {
        borrow_store_resource(&store).metadata
    }

    /// Return the `amount` of a given fungible asset.
    public fun amount(fa: &FungibleAsset): u64 {
        fa.amount
    }

    #[view]
    /// Get the balance of a given store.
    public fun balance<T: key>(store: Object<T>): u64 acquires FungibleStore {
        if (store_exists(object::object_address(&store))) {
            borrow_store_resource(&store).balance
        } else {
            0
        }
    }

    #[view]
    /// Return whether a store is frozen.
    ///
    /// If the store has not been created, we default to returning false so deposits can be sent to it.
    public fun is_frozen<T: key>(store: Object<T>): bool acquires FungibleStore {
        store_exists(object::object_address(&store)) && borrow_store_resource(&store).frozen
    }

    public fun asset_metadata(fa: &FungibleAsset): Object<Metadata> {
        fa.metadata
    }

    /// Get the underlying metadata object from the `MintRef`.
    public fun mint_ref_metadata(ref: &MintRef): Object<Metadata> {
        ref.metadata
    }

    /// Get the underlying metadata object from the `TransferRef`.
    public fun transfer_ref_metadata(ref: &TransferRef): Object<Metadata> {
        ref.metadata
    }

    /// Get the underlying metadata object from the `BurnRef`.
    public fun burn_ref_metadata(ref: &BurnRef): Object<Metadata> {
        ref.metadata
    }

    /// Transfer an `amount` of fungible asset from `from_store`, which should be owned by `sender`, to `receiver`.
    /// Note: it does not move the underlying object.
    public entry fun transfer<T: key>(
        sender: &signer,
        from: Object<T>,
        to: Object<T>,
        amount: u64,
    ) acquires FungibleStore, FungibleAssetEvents {
        let fa = withdraw(sender, from, amount);
        deposit(to, fa);
    }

    /// Allow an object to hold a store for fungible assets.
    /// Applications can use this to create multiple stores for isolating fungible assets for different purposes.
    public fun create_store<T: key>(
        constructor_ref: &ConstructorRef,
        metadata: Object<T>,
    ): Object<FungibleStore> {
        let store_obj = &object::generate_signer(constructor_ref);
        move_to(store_obj, FungibleStore {
            metadata: object::convert(metadata),
            balance: 0,
            frozen: false,
        });
        move_to(store_obj,
            FungibleAssetEvents {
                deposit_events: object::new_event_handle<DepositEvent>(store_obj),
                withdraw_events: object::new_event_handle<WithdrawEvent>(store_obj),
                frozen_events: object::new_event_handle<FrozenEvent>(store_obj),
            }
        );

        object::object_from_constructor_ref<FungibleStore>(constructor_ref)
    }

    /// Used to delete a store.  Requires the store to be completely empty prior to removing it
    public fun remove_store(delete_ref: &DeleteRef) acquires FungibleStore, FungibleAssetEvents {
        let store = &object::object_from_delete_ref<FungibleStore>(delete_ref);
        let addr = object::object_address(store);
        let FungibleStore { metadata: _, balance, frozen: _ }
            = move_from<FungibleStore>(addr);
        assert!(balance == 0, error::permission_denied(EBALANCE_IS_NOT_ZERO));
        let FungibleAssetEvents {
            deposit_events,
            withdraw_events,
            frozen_events,
        } = move_from<FungibleAssetEvents>(addr);
        event::destroy_handle(deposit_events);
        event::destroy_handle(withdraw_events);
        event::destroy_handle(frozen_events);
    }

    /// Withdraw `amount` of the fungible asset from `store` by the owner.
    public fun withdraw<T: key>(
        owner: &signer,
        store: Object<T>,
        amount: u64,
    ): FungibleAsset acquires FungibleStore, FungibleAssetEvents {
        assert!(object::owns(store, signer::address_of(owner)), error::permission_denied(ENOT_STORE_OWNER));
        assert!(!is_frozen(store), error::invalid_argument(ESTORE_IS_FROZEN));
        withdraw_internal(object::object_address(&store), amount)
    }

    /// Deposit `amount` of the fungible asset to `store`.
    public fun deposit<T: key>(store: Object<T>, fa: FungibleAsset) acquires FungibleStore, FungibleAssetEvents {
        assert!(!is_frozen(store), error::invalid_argument(ESTORE_IS_FROZEN));
        deposit_internal(store, fa);
    }

    /// Mint the specified `amount` of the fungible asset.
    public fun mint(ref: &MintRef, amount: u64): FungibleAsset acquires Supply, ConcurrentSupply {
        assert!(amount > 0, error::invalid_argument(EAMOUNT_CANNOT_BE_ZERO));
        let metadata = ref.metadata;
        increase_supply(&metadata, amount);

        FungibleAsset {
            metadata,
            amount
        }
    }

    /// Mint the specified `amount` of the fungible asset to a destination store.
    public fun mint_to<T: key>(ref: &MintRef, store: Object<T>, amount: u64)
    acquires FungibleStore, FungibleAssetEvents, Supply, ConcurrentSupply {
        deposit(store, mint(ref, amount));
    }

    /// Enable/disable a store's ability to do direct transfers of the fungible asset.
    public fun set_frozen_flag<T: key>(
        ref: &TransferRef,
        store: Object<T>,
        frozen: bool,
    ) acquires FungibleStore, FungibleAssetEvents {
        assert!(
            ref.metadata == store_metadata(store),
            error::invalid_argument(ETRANSFER_REF_AND_STORE_MISMATCH),
        );
        let store_addr = object::object_address(&store);
        borrow_global_mut<FungibleStore>(store_addr).frozen = frozen;

        let events = borrow_global_mut<FungibleAssetEvents>(store_addr);
        event::emit_event(&mut events.frozen_events, FrozenEvent { frozen });
    }

    /// Burns a fungible asset
    public fun burn(ref: &BurnRef, fa: FungibleAsset) acquires Supply, ConcurrentSupply {
        let FungibleAsset {
            metadata,
            amount,
        } = fa;
        assert!(ref.metadata == metadata, error::invalid_argument(EBURN_REF_AND_FUNGIBLE_ASSET_MISMATCH));
        decrease_supply(&metadata, amount);
    }

    /// Burn the `amount` of the fungible asset from the given store.
    public fun burn_from<T: key>(
        ref: &BurnRef,
        store: Object<T>,
        amount: u64
    ) acquires FungibleStore, FungibleAssetEvents, Supply, ConcurrentSupply {
        let metadata = ref.metadata;
        assert!(metadata == store_metadata(store), error::invalid_argument(EBURN_REF_AND_STORE_MISMATCH));
        let store_addr = object::object_address(&store);
        burn(ref, withdraw_internal(store_addr, amount));
    }

    /// Withdraw `amount` of the fungible asset from the `store` ignoring `frozen`.
    public fun withdraw_with_ref<T: key>(
        ref: &TransferRef,
        store: Object<T>,
        amount: u64
    ): FungibleAsset acquires FungibleStore, FungibleAssetEvents {
        assert!(
            ref.metadata == store_metadata(store),
            error::invalid_argument(ETRANSFER_REF_AND_STORE_MISMATCH),
        );
        withdraw_internal(object::object_address(&store), amount)
    }

    /// Deposit the fungible asset into the `store` ignoring `frozen`.
    public fun deposit_with_ref<T: key>(
        ref: &TransferRef,
        store: Object<T>,
        fa: FungibleAsset
    ) acquires FungibleStore, FungibleAssetEvents {
        assert!(
            ref.metadata == fa.metadata,
            error::invalid_argument(ETRANSFER_REF_AND_FUNGIBLE_ASSET_MISMATCH)
        );
        deposit_internal(store, fa);
    }

    /// Transfer `amount` of the fungible asset with `TransferRef` even it is frozen.
    public fun transfer_with_ref<T: key>(
        transfer_ref: &TransferRef,
        from: Object<T>,
        to: Object<T>,
        amount: u64,
    ) acquires FungibleStore, FungibleAssetEvents {
        let fa = withdraw_with_ref(transfer_ref, from, amount);
        deposit_with_ref(transfer_ref, to, fa);
    }

    /// Create a fungible asset with zero amount.
    /// This can be useful when starting a series of computations where the initial value is 0.
    public fun zero<T: key>(metadata: Object<T>): FungibleAsset {
        FungibleAsset {
            metadata: object::convert(metadata),
            amount: 0,
        }
    }

    /// Extract a given amount from the given fungible asset and return a new one.
    public fun extract(fungible_asset: &mut FungibleAsset, amount: u64): FungibleAsset {
        assert!(fungible_asset.amount >= amount, error::invalid_argument(EINSUFFICIENT_BALANCE));
        fungible_asset.amount = fungible_asset.amount - amount;
        FungibleAsset {
            metadata: fungible_asset.metadata,
            amount,
        }
    }

    /// "Merges" the two given fungible assets. The fungible asset passed in as `dst_fungible_asset` will have a value
    /// equal to the sum of the two (`dst_fungible_asset` and `src_fungible_asset`).
    public fun merge(dst_fungible_asset: &mut FungibleAsset, src_fungible_asset: FungibleAsset) {
        let FungibleAsset { metadata, amount } = src_fungible_asset;
        assert!(metadata == dst_fungible_asset.metadata, error::invalid_argument(EFUNGIBLE_ASSET_MISMATCH));
        dst_fungible_asset.amount = dst_fungible_asset.amount + amount;
    }

    /// Destroy an empty fungible asset.
    public fun destroy_zero(fungible_asset: FungibleAsset) {
        let FungibleAsset { amount, metadata: _ } = fungible_asset;
        assert!(amount == 0, error::invalid_argument(EAMOUNT_IS_NOT_ZERO));
    }

    fun deposit_internal<T: key>(store: Object<T>, fa: FungibleAsset) acquires FungibleStore, FungibleAssetEvents {
        let FungibleAsset { metadata, amount } = fa;
        if (amount == 0) return;

        let store_metadata = store_metadata(store);
        assert!(metadata == store_metadata, error::invalid_argument(EFUNGIBLE_ASSET_AND_STORE_MISMATCH));
        let store_addr = object::object_address(&store);
        let store = borrow_global_mut<FungibleStore>(store_addr);
        store.balance = store.balance + amount;

        let events = borrow_global_mut<FungibleAssetEvents>(store_addr);
        event::emit_event(&mut events.deposit_events, DepositEvent { amount });
    }

    /// Extract `amount` of the fungible asset from `store`.
    fun withdraw_internal(
        store_addr: address,
        amount: u64,
    ): FungibleAsset acquires FungibleStore, FungibleAssetEvents {
        assert!(amount != 0, error::invalid_argument(EAMOUNT_CANNOT_BE_ZERO));
        let store = borrow_global_mut<FungibleStore>(store_addr);
        assert!(store.balance >= amount, error::invalid_argument(EINSUFFICIENT_BALANCE));
        store.balance = store.balance - amount;

        let events = borrow_global_mut<FungibleAssetEvents>(store_addr);
        let metadata = store.metadata;
        event::emit_event(&mut events.withdraw_events, WithdrawEvent { amount });

        FungibleAsset { metadata, amount }
    }

    /// Increase the supply of a fungible asset by minting.
    fun increase_supply<T: key>(metadata: &Object<T>, amount: u64) acquires Supply, ConcurrentSupply {
        assert!(amount != 0, error::invalid_argument(EAMOUNT_CANNOT_BE_ZERO));
        let metadata_address = object::object_address(metadata);

        if (exists<ConcurrentSupply>(metadata_address)) {
            let supply = borrow_global_mut<ConcurrentSupply>(metadata_address);
            assert!(
                aggregator_v2::try_add(&mut supply.current, (amount as u128)),
                error::out_of_range(EMAX_SUPPLY_EXCEEDED)
            );
        } else if (exists<Supply>(metadata_address)) {
            let supply = borrow_global_mut<Supply>(metadata_address);
            if (option::is_some(&supply.maximum)) {
                let max = *option::borrow_mut(&mut supply.maximum);
                assert!(
                    max - supply.current >= (amount as u128),
                    error::out_of_range(EMAX_SUPPLY_EXCEEDED)
                )
            };
            supply.current = supply.current + (amount as u128);
        } else {
            assert!(false, error::not_found(ESUPPLY_NOT_FOUND));
        }
    }

    /// Decrease the supply of a fungible asset by burning.
    fun decrease_supply<T: key>(metadata: &Object<T>, amount: u64) acquires Supply, ConcurrentSupply {
        assert!(amount != 0, error::invalid_argument(EAMOUNT_CANNOT_BE_ZERO));
        let metadata_address = object::object_address(metadata);

        if (exists<ConcurrentSupply>(metadata_address)) {
            let supply = borrow_global_mut<ConcurrentSupply>(metadata_address);

            assert!(
                aggregator_v2::try_sub(&mut supply.current, (amount as u128)),
                error::out_of_range(ESUPPLY_UNDERFLOW)
            );
        } else if (exists<Supply>(metadata_address)) {
            assert!(exists<Supply>(metadata_address), error::not_found(ESUPPLY_NOT_FOUND));
            let supply = borrow_global_mut<Supply>(metadata_address);
            assert!(
                supply.current >= (amount as u128),
                error::invalid_state(ESUPPLY_UNDERFLOW)
            );
            supply.current = supply.current - (amount as u128);
        } else {
            assert!(false, error::not_found(ESUPPLY_NOT_FOUND));
        }
    }

    inline fun borrow_fungible_metadata<T: key>(
        metadata: &Object<T>
    ): &Metadata acquires Metadata {
        let addr = object::object_address(metadata);
        borrow_global<Metadata>(addr)
    }

    inline fun borrow_fungible_metadata_mut<T: key>(
        metadata: &Object<T>
    ): &mut Metadata acquires Metadata {
        let addr = object::object_address(metadata);
        borrow_global_mut<Metadata>(addr)
    }

    inline fun borrow_store_resource<T: key>(store: &Object<T>): &FungibleStore acquires FungibleStore {
        borrow_global<FungibleStore>(object::object_address(store))
    }

    public fun upgrade_to_concurrent(
        ref: &ExtendRef,
    ) acquires Supply {
        let metadata_object_address = object::address_from_extend_ref(ref);
        let metadata_object_signer = object::generate_signer_for_extending(ref);
        assert!(features::concurrent_assets_enabled(), error::invalid_argument(ECONCURRENT_SUPPLY_NOT_ENABLED));
        assert!(exists<Supply>(metadata_object_address), error::not_found(ESUPPLY_NOT_FOUND));
        let Supply {
            current,
            maximum,
        } = move_from<Supply>(metadata_object_address);

        let unlimited = option::is_none(&maximum);
        let supply = ConcurrentSupply {
            current: if (unlimited) {
                aggregator_v2::create_unbounded_aggregator()
            }
            else {
                aggregator_v2::create_aggregator(option::extract(&mut maximum))
            },
        };
        // update current state:
        aggregator_v2::add(&mut supply.current, current);
        move_to(&metadata_object_signer, supply);
    }

    #[test_only]
    use aptos_framework::account;

    #[test_only]
    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]

    struct TestToken has key {}

    #[test_only]
    public fun create_test_token(creator: &signer): (ConstructorRef, Object<TestToken>) {
        account::create_account_for_test(signer::address_of(creator));
        let creator_ref = object::create_named_object(creator, b"TEST");
        let object_signer = object::generate_signer(&creator_ref);
        move_to(&object_signer, TestToken {});

        let token = object::object_from_constructor_ref<TestToken>(&creator_ref);
        (creator_ref, token)
    }

    #[test_only]
    public fun init_test_metadata(constructor_ref: &ConstructorRef): (MintRef, TransferRef, BurnRef) {
        add_fungibility(
            constructor_ref,
            option::some(100) /* max supply */,
            string::utf8(b"TEST"),
            string::utf8(b"@@"),
            0,
            string::utf8(b"http://www.example.com/favicon.ico"),
            string::utf8(b"http://www.example.com"),
        );
        let mint_ref = generate_mint_ref(constructor_ref);
        let burn_ref = generate_burn_ref(constructor_ref);
        let transfer_ref = generate_transfer_ref(constructor_ref);
        (mint_ref, transfer_ref, burn_ref)
    }

    #[test_only]
    public fun create_fungible_asset(
        creator: &signer
    ): (MintRef, TransferRef, BurnRef, Object<Metadata>) {
        let (creator_ref, token_object) = create_test_token(creator);
        let (mint, transfer, burn) = init_test_metadata(&creator_ref);
        (mint, transfer, burn, object::convert(token_object))
    }

    #[test_only]
    public fun create_test_store<T: key>(owner: &signer, metadata: Object<T>): Object<FungibleStore> {
        let owner_addr = signer::address_of(owner);
        if (!account::exists_at(owner_addr)) {
            account::create_account_for_test(owner_addr);
        };
        create_store(&object::create_object_from_account(owner), metadata)
    }

    #[test(creator = @0xcafe)]
    fun test_metadata_basic_flow(creator: &signer) acquires Metadata, Supply, ConcurrentSupply {
        let (creator_ref, metadata) = create_test_token(creator);
        init_test_metadata(&creator_ref);
        assert!(supply(metadata) == option::some(0), 1);
        assert!(maximum(metadata) == option::some(100), 2);
        assert!(name(metadata) == string::utf8(b"TEST"), 3);
        assert!(symbol(metadata) == string::utf8(b"@@"), 4);
        assert!(decimals(metadata) == 0, 5);

        increase_supply(&metadata, 50);
        assert!(supply(metadata) == option::some(50), 6);
        decrease_supply(&metadata, 30);
        assert!(supply(metadata) == option::some(20), 7);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 0x20005, location = Self)]
    fun test_supply_overflow(creator: &signer) acquires Supply, ConcurrentSupply {
        let (creator_ref, metadata) = create_test_token(creator);
        init_test_metadata(&creator_ref);
        increase_supply(&metadata, 101);
    }

    #[test(creator = @0xcafe)]
    fun test_create_and_remove_store(creator: &signer) acquires FungibleStore, FungibleAssetEvents {
        let (_, _, _, metadata) = create_fungible_asset(creator);
        let creator_ref = object::create_object_from_account(creator);
        create_store(&creator_ref, metadata);
        let delete_ref = object::generate_delete_ref(&creator_ref);
        remove_store(&delete_ref);
    }

    #[test(creator = @0xcafe, aaron = @0xface)]
    fun test_e2e_basic_flow(
        creator: &signer,
        aaron: &signer,
    ) acquires FungibleStore, FungibleAssetEvents, Supply, ConcurrentSupply {
        let (mint_ref, transfer_ref, burn_ref, test_token) = create_fungible_asset(creator);
        let metadata = mint_ref.metadata;
        let creator_store = create_test_store(creator, metadata);
        let aaron_store = create_test_store(aaron, metadata);

        assert!(supply(test_token) == option::some(0), 1);
        // Mint
        let fa = mint(&mint_ref, 100);
        assert!(supply(test_token) == option::some(100), 2);
        // Deposit
        deposit(creator_store, fa);
        // Withdraw
        let fa = withdraw(creator, creator_store, 80);
        assert!(supply(test_token) == option::some(100), 3);
        deposit(aaron_store, fa);
        // Burn
        burn_from(&burn_ref, aaron_store, 30);
        assert!(supply(test_token) == option::some(70), 4);
        // Transfer
        transfer(creator, creator_store, aaron_store, 10);
        assert!(balance(creator_store) == 10, 5);
        assert!(balance(aaron_store) == 60, 6);

        set_frozen_flag(&transfer_ref, aaron_store, true);
        assert!(is_frozen(aaron_store), 7);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 0x10003, location = Self)]
    fun test_frozen(
        creator: &signer
    ) acquires FungibleStore, FungibleAssetEvents, Supply, ConcurrentSupply {
        let (mint_ref, transfer_ref, _burn_ref, _) = create_fungible_asset(creator);

        let creator_store = create_test_store(creator, mint_ref.metadata);
        let fa = mint(&mint_ref, 100);
        set_frozen_flag(&transfer_ref, creator_store, true);
        deposit(creator_store, fa);
    }

    #[test(creator = @0xcafe, aaron = @0xface)]
    fun test_transfer_with_ref(
        creator: &signer,
        aaron: &signer,
    ) acquires FungibleStore, FungibleAssetEvents, Supply, ConcurrentSupply {
        let (mint_ref, transfer_ref, _burn_ref, _) = create_fungible_asset(creator);
        let metadata = mint_ref.metadata;
        let creator_store = create_test_store(creator, metadata);
        let aaron_store = create_test_store(aaron, metadata);

        let fa = mint(&mint_ref, 100);
        set_frozen_flag(&transfer_ref, creator_store, true);
        set_frozen_flag(&transfer_ref, aaron_store, true);
        deposit_with_ref(&transfer_ref, creator_store, fa);
        transfer_with_ref(&transfer_ref, creator_store, aaron_store, 80);
        assert!(balance(creator_store) == 20, 1);
        assert!(balance(aaron_store) == 80, 2);
        assert!(!!is_frozen(creator_store), 3);
        assert!(!!is_frozen(aaron_store), 4);
    }

    #[test(creator = @0xcafe)]
    fun test_merge_and_exact(creator: &signer) acquires Supply, ConcurrentSupply {
        let (mint_ref, _transfer_ref, burn_ref, _) = create_fungible_asset(creator);
        let fa = mint(&mint_ref, 100);
        let cash = extract(&mut fa, 80);
        assert!(fa.amount == 20, 1);
        assert!(cash.amount == 80, 2);
        let more_cash = extract(&mut fa, 20);
        destroy_zero(fa);
        merge(&mut cash, more_cash);
        assert!(cash.amount == 100, 3);
        burn(&burn_ref, cash);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 0x10012, location = Self)]
    fun test_add_fungibility_to_deletable_object(creator: &signer) {
        account::create_account_for_test(signer::address_of(creator));
        let creator_ref = &object::create_object_from_account(creator);
        init_test_metadata(creator_ref);
    }

    #[test(creator = @0xcafe, aaron = @0xface)]
    #[expected_failure(abort_code = 0x10006, location = Self)]
    fun test_fungible_asset_mismatch_when_merge(creator: &signer, aaron: &signer) {
        let (_, _, _, metadata1) = create_fungible_asset(creator);
        let (_, _, _, metadata2) = create_fungible_asset(aaron);
        let base = FungibleAsset {
            metadata: metadata1,
            amount: 1,
        };
        let addon = FungibleAsset {
            metadata: metadata2,
            amount: 1
        };
        merge(&mut base, addon);
        let FungibleAsset {
            metadata: _,
            amount: _
        } = base;
    }

    #[test(fx = @aptos_framework, creator = @0xcafe)]
    fun test_fungible_asset_upgrade(fx: &signer, creator: &signer) acquires Supply, ConcurrentSupply, FungibleAssetEvents, FungibleStore {
        let feature = features::get_concurrent_assets_feature();
        features::change_feature_flags(fx, vector[], vector[feature]);

        let (creator_ref, token_object) = create_test_token(creator);
        let (mint_ref, transfer_ref, _burn) = init_test_metadata(&creator_ref);
        let test_token = object::convert<TestToken, Metadata>(token_object);
        let creator_store = create_test_store(creator, test_token);

        let fa = mint(&mint_ref, 30);
        assert!(supply(test_token) == option::some(30), 2);

        deposit_with_ref(&transfer_ref, creator_store, fa);

        features::change_feature_flags(fx, vector[feature], vector[]);

        let extend_ref = object::generate_extend_ref(&creator_ref);
        upgrade_to_concurrent(&extend_ref);

        let fb = mint(&mint_ref, 20);
        assert!(supply(test_token) == option::some(50), 3);

        deposit_with_ref(&transfer_ref, creator_store, fb);
    }
}
