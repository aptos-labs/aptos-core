/// This module provides the foundation for typesafe Coins.
module aptos_framework::coin {
    use std::string;
    use std::error;
    use std::option::{Self, Option};
    use std::signer;
    use aptos_std::smart_table;
    use aptos_std::smart_table::SmartTable;

    use aptos_framework::aggregator_factory;
    use aptos_framework::aggregator::{Self, Aggregator};
    use aptos_framework::event::{Self, EventHandle};
    use aptos_framework::optional_aggregator::{Self, OptionalAggregator};
    use aptos_framework::system_addresses;

    use aptos_framework::fungible_asset::{Self, FungibleAsset, Metadata, MintRef, TransferRef, BurnRef};
    use aptos_framework::object::{Self, Object};
    use aptos_framework::primary_fungible_store;
    use aptos_std::type_info::{Self, TypeInfo};
    use aptos_framework::create_signer;
    #[test_only]
    use aptos_framework::account;

    friend aptos_framework::aptos_coin;
    friend aptos_framework::genesis;
    friend aptos_framework::transaction_fee;

    //
    // Errors.
    //

    /// Address of account which is used to initialize a coin `CoinType` doesn't match the deployer of module
    const ECOIN_INFO_ADDRESS_MISMATCH: u64 = 1;

    /// `CoinType` is already initialized as a coin
    const ECOIN_INFO_ALREADY_PUBLISHED: u64 = 2;

    /// `CoinType` hasn't been initialized as a coin
    const ECOIN_INFO_NOT_PUBLISHED: u64 = 3;

    /// Deprecated. Account already has `CoinStore` registered for `CoinType`
    const ECOIN_STORE_ALREADY_PUBLISHED: u64 = 4;

    /// Account hasn't registered `CoinStore` for `CoinType`
    const ECOIN_STORE_NOT_PUBLISHED: u64 = 5;

    /// Not enough coins to complete transaction
    const EINSUFFICIENT_BALANCE: u64 = 6;

    /// Cannot destroy non-zero coins
    const EDESTRUCTION_OF_NONZERO_TOKEN: u64 = 7;

    /// CoinStore is frozen. Coins cannot be deposited or withdrawn
    const EFROZEN: u64 = 10;

    /// Cannot upgrade the total supply of coins to different implementation.
    const ECOIN_SUPPLY_UPGRADE_NOT_SUPPORTED: u64 = 11;

    /// Name of the coin is too long
    const ECOIN_NAME_TOO_LONG: u64 = 12;

    /// Symbol of the coin is too long
    const ECOIN_SYMBOL_TOO_LONG: u64 = 13;

    /// The value of aggregatable coin used for transaction fees redistribution does not fit in u64.
    const EAGGREGATABLE_COIN_VALUE_TOO_LARGE: u64 = 14;

    /// The coin and fungible asset conversion map does not exist.
    const ECOIN_FUNGIBLE_ASSET_MAP: u64 = 15;

    /// The coin type does not exist as a key in the conversion map.
    const ECOIN_TYPE_AS_KEY: u64 = 16;

    /// The fungible asset metadata does not exist as a key in the conversion map.
    const EFUNGIBLE_ASSET_METADATA_AS_KEY: u64 = 17;

    /// The coin type from the map does not match the calling function type argument.
    const ECOIN_TYPE_MISMATCH: u64 = 18;

    //
    // Constants
    //

    const MAX_COIN_NAME_LENGTH: u64 = 32;
    const MAX_COIN_SYMBOL_LENGTH: u64 = 10;

    /// Core data structures

    /// Main structure representing a coin/token in an account's custody.
    struct Coin<phantom CoinType> has store {
        /// Amount of coin this address has.
        value: u64,
    }

    /// Represents a coin with aggregator as its value. This allows to update
    /// the coin in every transaction avoiding read-modify-write conflicts. Only
    /// used for gas fees distribution by Aptos Framework (0x1).
    struct AggregatableCoin<phantom CoinType> has store {
        /// Amount of aggregatable coin this address has.
        value: Aggregator,
    }

    /// Maximum possible aggregatable coin value.
    const MAX_U64: u128 = 18446744073709551615;

    /// A holder of a specific coin types and associated event handles.
    /// These are kept in a single resource to ensure locality of data.
    struct CoinStore<phantom CoinType> has key {
        coin: Coin<CoinType>,
        frozen: bool,
        deposit_events: EventHandle<DepositEvent>,
        withdraw_events: EventHandle<WithdrawEvent>,
    }

    /// Maximum possible coin supply.
    const MAX_U128: u128 = 340282366920938463463374607431768211455;

    /// Configuration that controls the behavior of total coin supply. If the field
    /// is set, coin creators are allowed to upgrade to parallelizable implementations.
    struct SupplyConfig has key {
        allow_upgrades: bool,
    }

    /// Information about a specific coin type. Stored on the creator of the coin's account.
    struct CoinInfo<phantom CoinType> has key {
        name: string::String,
        /// Symbol of the coin, usually a shorter version of the name.
        /// For example, Singapore Dollar is SGD.
        symbol: string::String,
        /// Number of decimals used to get its user representation.
        /// For example, if `decimals` equals `2`, a balance of `505` coins should
        /// be displayed to a user as `5.05` (`505 / 10 ** 2`).
        decimals: u8,
        /// Amount of this coin type in existence.
        supply: Option<OptionalAggregator>,
    }

    /// Event emitted when some amount of a coin is deposited into an account.
    struct DepositEvent has drop, store {
        amount: u64,
    }

    /// Event emitted when some amount of a coin is withdrawn from an account.
    struct WithdrawEvent has drop, store {
        amount: u64,
    }

    /// Capability required to mint coins.
    struct MintCapability<phantom CoinType> has copy, store {}

    /// Capability required to freeze a coin store.
    struct FreezeCapability<phantom CoinType> has copy, store {}

    /// Capability required to burn coins.
    struct BurnCapability<phantom CoinType> has copy, store {}

    /// The mapping between coin and fungible asset.
    struct CoinConversionMap has key {
        coin_to_fungible_asset_map: SmartTable<TypeInfo, address>,
        fungible_asset_to_coin_map: SmartTable<address, TypeInfo>,
    }

    inline fun borrow_conversion_map(): &CoinConversionMap {
        if (!exists<CoinConversionMap>(@aptos_framework)) {
            move_to(&create_signer::create_signer(@aptos_framework), CoinConversionMap {
                coin_to_fungible_asset_map: smart_table::new<TypeInfo, address>(),
                fungible_asset_to_coin_map: smart_table::new<address, TypeInfo>(),
            })
        };
        borrow_global<CoinConversionMap>(@aptos_framework)
    }

    inline fun borrow_conversion_map_mut(): &mut CoinConversionMap {
        if (!exists<CoinConversionMap>(@aptos_framework)) {
            move_to(&create_signer::create_signer(@aptos_framework), CoinConversionMap {
                coin_to_fungible_asset_map: smart_table::new<TypeInfo, address>(),
                fungible_asset_to_coin_map: smart_table::new<address, TypeInfo>(),
            })
        };
        borrow_global_mut<CoinConversionMap>(@aptos_framework)
    }

    /// Get the paired fungible asset metadata object of a coin type, create if not exist.
    public fun paired_metadata<CoinType>(): Object<Metadata> acquires CoinConversionMap, CoinInfo {
        let map = borrow_conversion_map_mut();
        let type = type_info::type_of<CoinType>();
        if (!smart_table::contains(&map.coin_to_fungible_asset_map, type)) {
            let metadata_object_cref =
                if (type_info::type_name<CoinType>() == string::utf8(b"0x1::aptos_coin::AptosCoin")) {
                    object::create_object_at_address(@aptos_framework, @aptos_framework, false)
                } else {
                    object::create_sticky_object(coin_address<CoinType>())
                };
            primary_fungible_store::create_primary_store_enabled_fungible_asset(
                &metadata_object_cref,
                option::map<u128, u128>(coin_supply<CoinType>(), |_| MAX_U128),
                name<CoinType>(),
                symbol<CoinType>(),
                decimals<CoinType>(),
                string::utf8(b""),
                string::utf8(b""),
            );
            let metadata_addr = object::address_from_constructor_ref(&metadata_object_cref);
            smart_table::add(&mut map.coin_to_fungible_asset_map, type, metadata_addr);
            smart_table::add(&mut map.fungible_asset_to_coin_map, metadata_addr, type);
        };
        object::address_to_object<Metadata>(*smart_table::borrow(&map.coin_to_fungible_asset_map, type))
    }

    // Conversion from coin to fungible asset
    public fun coin_to_fungible_asset<CoinType>(
        coin: Coin<CoinType>
    ): FungibleAsset acquires CoinConversionMap, CoinInfo {
        let metadata = paired_metadata<CoinType>();
        let amount = burn_internal(coin);
        fungible_asset::mint_internal(metadata, amount)
    }

    // Conversion from fungible asset to coin. Not public to push the migration to FA.
    fun fungible_asset_to_coin<CoinType>(
        fungible_asset: FungibleAsset
    ): Coin<CoinType> acquires CoinConversionMap, CoinInfo {
        let map = borrow_conversion_map();
        let metadata = object::object_address(&fungible_asset::metadata_from_asset(&fungible_asset));
        assert!(
            smart_table::contains(&map.fungible_asset_to_coin_map, metadata),
            error::not_found(EFUNGIBLE_ASSET_METADATA_AS_KEY)
        );
        let coin_type_info = *smart_table::borrow(&map.fungible_asset_to_coin_map, metadata);
        assert!(coin_type_info == type_info::type_of<CoinType>(), error::invalid_argument(ECOIN_TYPE_MISMATCH));
        let amount = fungible_asset::burn_internal(fungible_asset);
        mint_internal<CoinType>(amount)
    }

    /// Get the `MintRef` of paired fungible asset of a coin type from `MintCapability`.
    public fun paired_mint_ref<CoinType>(_: &MintCapability<CoinType>): MintRef acquires CoinConversionMap, CoinInfo {
        let metadata = paired_metadata<CoinType>();
        fungible_asset::get_mint_ref_internal(metadata)
    }

    /// Get the TransferRef of paired fungible asset of a coin type from `FreezeCapability`.
    public fun paired_transfer_ref<CoinType>(
        _: &FreezeCapability<CoinType>
    ): TransferRef acquires CoinConversionMap, CoinInfo {
        let metadata = paired_metadata<CoinType>();
        fungible_asset::get_transfer_ref_internal(metadata)
    }

    /// Get the `BurnRef` of paired fungible asset of a coin type from `BurnCapability`.
    public fun paired_burn_ref<CoinType>(_: &BurnCapability<CoinType>): BurnRef acquires CoinConversionMap, CoinInfo {
        let metadata = paired_metadata<CoinType>();
        fungible_asset::get_burn_ref_internal(metadata)
    }

    //
    // Total supply config
    //

    /// Publishes supply configuration. Initially, upgrading is not allowed.
    public(friend) fun initialize_supply_config(aptos_framework: &signer) {
        system_addresses::assert_aptos_framework(aptos_framework);
        move_to(aptos_framework, SupplyConfig { allow_upgrades: false });
    }

    /// This should be called by on-chain governance to update the config and allow
    /// or disallow upgradability of total supply.
    public fun allow_supply_upgrades(aptos_framework: &signer, allowed: bool) acquires SupplyConfig {
        system_addresses::assert_aptos_framework(aptos_framework);
        let allow_upgrades = &mut borrow_global_mut<SupplyConfig>(@aptos_framework).allow_upgrades;
        *allow_upgrades = allowed;
    }

    //
    //  Aggregatable coin functions
    //

    /// Creates a new aggregatable coin with value overflowing on `limit`. Note that this function can
    /// only be called by Aptos Framework (0x1) account for now because of `create_aggregator`.
    public(friend) fun initialize_aggregatable_coin<CoinType>(aptos_framework: &signer): AggregatableCoin<CoinType> {
        let aggregator = aggregator_factory::create_aggregator(aptos_framework, MAX_U64);
        AggregatableCoin<CoinType> {
            value: aggregator,
        }
    }

    /// Returns true if the value of aggregatable coin is zero.
    public(friend) fun is_aggregatable_coin_zero<CoinType>(coin: &AggregatableCoin<CoinType>): bool {
        let amount = aggregator::read(&coin.value);
        amount == 0
    }

    /// Drains the aggregatable coin, setting it to zero and returning a standard coin.
    public(friend) fun drain_aggregatable_coin<CoinType>(coin: &mut AggregatableCoin<CoinType>): Coin<CoinType> {
        spec {
            // TODO: The data invariant is not properly assumed from CollectedFeesPerBlock.
            assume aggregator::spec_get_limit(coin.value) == MAX_U64;
        };
        let amount = aggregator::read(&coin.value);
        assert!(amount <= MAX_U64, error::out_of_range(EAGGREGATABLE_COIN_VALUE_TOO_LARGE));
        spec {
            update aggregate_supply<CoinType> = aggregate_supply<CoinType> - amount;
        };
        aggregator::sub(&mut coin.value, amount);
        spec {
            update supply<CoinType> = supply<CoinType> + amount;
        };
        Coin<CoinType> {
            value: (amount as u64),
        }
    }

    /// Merges `coin` into aggregatable coin (`dst_coin`).
    public(friend) fun merge_aggregatable_coin<CoinType>(
        dst_coin: &mut AggregatableCoin<CoinType>,
        coin: Coin<CoinType>
    ) {
        spec {
            update supply<CoinType> = supply<CoinType> - coin.value;
        };
        let Coin { value } = coin;
        let amount = (value as u128);
        spec {
            update aggregate_supply<CoinType> = aggregate_supply<CoinType> + amount;
        };
        aggregator::add(&mut dst_coin.value, amount);
    }

    /// Collects a specified amount of coin form an account into aggregatable coin.
    public(friend) fun collect_into_aggregatable_coin<CoinType>(
        account_addr: address,
        amount: u64,
        dst_coin: &mut AggregatableCoin<CoinType>,
    ) acquires CoinStore, CoinConversionMap, CoinInfo {
        // Skip collecting if amount is zero.
        if (amount == 0) {
            return
        };
        maybe_convert_to_fungible_store<CoinType>(account_addr);
        let coin = {
            let store = primary_fungible_store::primary_store(account_addr, paired_metadata<CoinType>());
            let fa = fungible_asset::withdraw_internal(object::object_address(&store), amount);
            fungible_asset_to_coin<CoinType>(fa)
        };
        merge_aggregatable_coin(dst_coin, coin);
    }

    fun maybe_convert_to_fungible_store<CoinType>(
        account: address
    ) acquires CoinStore, CoinConversionMap, CoinInfo {
        if (exists<CoinStore<CoinType>>(account)) {
            let CoinStore<CoinType> {
                coin,
                frozen,
                deposit_events,
                withdraw_events
            } = move_from<CoinStore<CoinType>>(account);
            event::destory_handle(deposit_events);
            event::destory_handle(withdraw_events);
            let fungible_asset = coin_to_fungible_asset(coin);
            let metadata = fungible_asset::asset_metadata(&fungible_asset);
            let store = primary_fungible_store::ensure_primary_store_exists(account, metadata);
            fungible_asset::deposit(store, fungible_asset);
            // Note:
            // It is possible the primary fungible store may already exist before this function call.
            // In this case, if the account owns a frozen CoinStore and an unfrozen primary fungible store, this
            // function would convert and deposit the rest coin into the primary store and freeze it to make the
            // `frozen` semantic as consistent as possible.
            fungible_asset::set_frozen_flag_internal(store, frozen);
        };
    }

    //
    // Getter functions
    //

    /// A helper function that returns the address of CoinType.
    fun coin_address<CoinType>(): address {
        let type_info = type_info::type_of<CoinType>();
        type_info::account_address(&type_info)
    }

    #[view]
    /// Returns the balance of `owner` for provided `CoinType` and its paired FA if exists.
    public fun balance<CoinType>(owner: address): u64 acquires CoinConversionMap, CoinInfo, CoinStore {
        maybe_convert_to_fungible_store<CoinType>(owner);
        primary_fungible_store::balance(owner, paired_metadata<CoinType>())
    }

    #[view]
    /// Returns `true` if the type `CoinType` is an initialized coin.
    public fun is_coin_initialized<CoinType>(): bool {
        exists<CoinInfo<CoinType>>(coin_address<CoinType>())
    }

    #[view]
    /// Returns `true` is account_addr has frozen the CoinStore or if it's not registered at all
    public fun is_coin_store_frozen<CoinType>(
        account_addr: address
    ): bool acquires CoinStore, CoinConversionMap, CoinInfo {
        maybe_convert_to_fungible_store<CoinType>(account_addr);
        primary_fungible_store::is_frozen(account_addr, paired_metadata<CoinType>())
    }

    #[deprecated]
    #[view]
    /// Always return `true` since `account_addr` can receive `CoinType` in fungible asset without registration.
    public fun is_account_registered<CoinType>(_account_addr: address): bool {
        true
    }

    #[view]
    /// Returns the name of the coin.
    public fun name<CoinType>(): string::String acquires CoinInfo {
        borrow_global<CoinInfo<CoinType>>(coin_address<CoinType>()).name
    }

    #[view]
    /// Returns the symbol of the coin, usually a shorter version of the name.
    public fun symbol<CoinType>(): string::String acquires CoinInfo {
        borrow_global<CoinInfo<CoinType>>(coin_address<CoinType>()).symbol
    }

    #[view]
    /// Returns the number of decimals used to get its user representation.
    /// For example, if `decimals` equals `2`, a balance of `505` coins should
    /// be displayed to a user as `5.05` (`505 / 10 ** 2`).
    public fun decimals<CoinType>(): u8 acquires CoinInfo {
        borrow_global<CoinInfo<CoinType>>(coin_address<CoinType>()).decimals
    }

    #[view]
    /// Returns the amount of coin in existence.
    public fun supply<CoinType>(): Option<u128> acquires CoinInfo, CoinConversionMap {
        let coin_supply = coin_supply<CoinType>();
        let metadata = paired_metadata<CoinType>();
        let fungible_asset_supply = fungible_asset::supply(metadata);
        if (option::is_some(&coin_supply)) {
            let supply = option::borrow_mut(&mut coin_supply);
            *supply = *supply + option::destroy_some(fungible_asset_supply);
        };
        coin_supply
    }

    #[view]
    /// Returns the amount of coin in existence.
    public fun coin_supply<CoinType>(): Option<u128> acquires CoinInfo {
        let maybe_supply = &borrow_global<CoinInfo<CoinType>>(coin_address<CoinType>()).supply;
        if (option::is_some(maybe_supply)) {
            // We do track supply, in this case read from optional aggregator.
            let supply = option::borrow(maybe_supply);
            let value = optional_aggregator::read(supply);
            option::some(value)
        } else {
            option::none()
        }
    }
    //
    // Public functions
    //

    /// Burn `coin` with capability.
    /// The capability `_cap` should be passed as a reference to `BurnCapability<CoinType>`.
    public fun burn<CoinType>(
        coin: Coin<CoinType>,
        _cap: &BurnCapability<CoinType>,
    ) acquires CoinInfo {
        burn_internal(coin);
    }

    /// Burn `coin` from the specified `account` with capability.
    /// The capability `burn_cap` should be passed as a reference to `BurnCapability<CoinType>`.
    /// This function shouldn't fail as it's called as part of transaction fee burning.
    ///
    /// Note: This bypasses CoinStore::frozen -- coins within a frozen CoinStore can be burned.
    public fun burn_from<CoinType>(
        account_addr: address,
        amount: u64,
        _burn_cap: &BurnCapability<CoinType>,
    ) acquires CoinInfo, CoinStore, CoinConversionMap {
        // Skip burning if amount is zero. This shouldn't error out as it's called as part of transaction fee burning.
        if (amount == 0) {
            return
        };
        maybe_convert_to_fungible_store<CoinType>(account_addr);
        let store = primary_fungible_store::primary_store(account_addr, paired_metadata<CoinType>());
        let fa = fungible_asset::withdraw_internal(object::object_address(&store), amount);
        fungible_asset::burn_internal(fa);
    }

    /// Deposit the coin balance into the recipient's account and emit an event.
    public fun deposit<CoinType>(
        account_addr: address,
        coin: Coin<CoinType>
    ) acquires CoinStore, CoinConversionMap, CoinInfo {
        maybe_convert_to_fungible_store<CoinType>(account_addr);
        primary_fungible_store::deposit(
            account_addr,
            coin_to_fungible_asset(coin)
        );
    }

    /// Deposit the coin balance into the recipient's account without checking if the account is frozen.
    /// This is for internal use only and doesn't emit an DepositEvent.
    public(friend) fun force_deposit<CoinType>(
        account_addr: address,
        coin: Coin<CoinType>
    ) acquires CoinStore, CoinConversionMap, CoinInfo {
        maybe_convert_to_fungible_store<CoinType>(account_addr);
        let fa = coin_to_fungible_asset(coin);
        let metadata = fungible_asset::asset_metadata(&fa);
        let store = primary_fungible_store::ensure_primary_store_exists(account_addr, metadata);
        fungible_asset::deposit_internal(store, fa);
    }

    /// Destroys a zero-value coin. Calls will fail if the `value` in the passed-in `token` is non-zero
    /// so it is impossible to "burn" any non-zero amount of `Coin` without having
    /// a `BurnCapability` for the specific `CoinType`.
    public fun destroy_zero<CoinType>(zero_coin: Coin<CoinType>) {
        spec {
            update supply<CoinType> = supply<CoinType> - zero_coin.value;
        };
        let Coin { value } = zero_coin;
        assert!(value == 0, error::invalid_argument(EDESTRUCTION_OF_NONZERO_TOKEN))
    }

    /// Extracts `amount` from the passed-in `coin`, where the original token is modified in place.
    public fun extract<CoinType>(coin: &mut Coin<CoinType>, amount: u64): Coin<CoinType> {
        assert!(coin.value >= amount, error::invalid_argument(EINSUFFICIENT_BALANCE));
        spec {
            update supply<CoinType> = supply<CoinType> - amount;
        };
        coin.value = coin.value - amount;
        spec {
            update supply<CoinType> = supply<CoinType> + amount;
        };
        Coin { value: amount }
    }

    /// Extracts the entire amount from the passed-in `coin`, where the original token is modified in place.
    public fun extract_all<CoinType>(coin: &mut Coin<CoinType>): Coin<CoinType> {
        let total_value = coin.value;
        spec {
            update supply<CoinType> = supply<CoinType> - coin.value;
        };
        coin.value = 0;
        spec {
            update supply<CoinType> = supply<CoinType> + total_value;
        };
        Coin { value: total_value }
    }

    #[legacy_entry_fun]
    /// Freeze a CoinStore to prevent transfers
    public entry fun freeze_coin_store<CoinType>(
        account_addr: address,
        _freeze_cap: &FreezeCapability<CoinType>,
    ) acquires CoinStore, CoinConversionMap, CoinInfo {
        maybe_convert_to_fungible_store<CoinType>(account_addr);
        let tmp_transfer_ref = &paired_transfer_ref(_freeze_cap);
        primary_fungible_store::set_frozen_flag(tmp_transfer_ref, account_addr, true)
    }

    #[legacy_entry_fun]
    /// Unfreeze a CoinStore to allow transfers
    public entry fun unfreeze_coin_store<CoinType>(
        account_addr: address,
        _freeze_cap: &FreezeCapability<CoinType>,
    ) acquires CoinStore, CoinConversionMap, CoinInfo {
        maybe_convert_to_fungible_store<CoinType>(account_addr);
        let tmp_transfer_ref = &paired_transfer_ref(_freeze_cap);
        primary_fungible_store::set_frozen_flag(tmp_transfer_ref, account_addr, false)
    }

    /// Upgrade total supply to use a parallelizable implementation if it is
    /// available.
    public entry fun upgrade_supply<CoinType>(account: &signer) acquires CoinInfo, SupplyConfig {
        let account_addr = signer::address_of(account);

        // Only coin creators can upgrade total supply.
        assert!(
            coin_address<CoinType>() == account_addr,
            error::invalid_argument(ECOIN_INFO_ADDRESS_MISMATCH),
        );

        // Can only succeed once on-chain governance agreed on the upgrade.
        assert!(
            borrow_global_mut<SupplyConfig>(@aptos_framework).allow_upgrades,
            error::permission_denied(ECOIN_SUPPLY_UPGRADE_NOT_SUPPORTED)
        );

        let maybe_supply = &mut borrow_global_mut<CoinInfo<CoinType>>(account_addr).supply;
        if (option::is_some(maybe_supply)) {
            let supply = option::borrow_mut(maybe_supply);

            // If supply is tracked and the current implementation uses an integer - upgrade.
            if (!optional_aggregator::is_parallelizable(supply)) {
                optional_aggregator::switch(supply);
            }
        }
    }

    /// Creates a new Coin with given `CoinType` and returns minting/freezing/burning capabilities.
    /// The given signer also becomes the account hosting the information  about the coin
    /// (name, supply, etc.). Supply is initialized as non-parallelizable integer.
    public fun initialize<CoinType>(
        account: &signer,
        name: string::String,
        symbol: string::String,
        decimals: u8,
        monitor_supply: bool,
    ): (BurnCapability<CoinType>, FreezeCapability<CoinType>, MintCapability<CoinType>) {
        initialize_internal(account, name, symbol, decimals, monitor_supply, false)
    }

    /// Same as `initialize` but supply can be initialized to parallelizable aggregator.
    public(friend) fun initialize_with_parallelizable_supply<CoinType>(
        account: &signer,
        name: string::String,
        symbol: string::String,
        decimals: u8,
        monitor_supply: bool,
    ): (BurnCapability<CoinType>, FreezeCapability<CoinType>, MintCapability<CoinType>) {
        system_addresses::assert_aptos_framework(account);
        initialize_internal(account, name, symbol, decimals, monitor_supply, true)
    }

    fun initialize_internal<CoinType>(
        account: &signer,
        name: string::String,
        symbol: string::String,
        decimals: u8,
        monitor_supply: bool,
        parallelizable: bool,
    ): (BurnCapability<CoinType>, FreezeCapability<CoinType>, MintCapability<CoinType>) {
        let account_addr = signer::address_of(account);

        assert!(
            coin_address<CoinType>() == account_addr,
            error::invalid_argument(ECOIN_INFO_ADDRESS_MISMATCH),
        );

        assert!(
            !exists<CoinInfo<CoinType>>(account_addr),
            error::already_exists(ECOIN_INFO_ALREADY_PUBLISHED),
        );

        assert!(string::length(&name) <= MAX_COIN_NAME_LENGTH, error::invalid_argument(ECOIN_NAME_TOO_LONG));
        assert!(string::length(&symbol) <= MAX_COIN_SYMBOL_LENGTH, error::invalid_argument(ECOIN_SYMBOL_TOO_LONG));

        let coin_info = CoinInfo<CoinType> {
            name,
            symbol,
            decimals,
            supply: if (monitor_supply) {
                option::some(
                    optional_aggregator::new(MAX_U128, parallelizable)
                )
            } else { option::none() },
        };
        move_to(account, coin_info);

        (BurnCapability<CoinType> {}, FreezeCapability<CoinType> {}, MintCapability<CoinType> {})
    }

    /// "Merges" the two given coins.  The coin passed in as `dst_coin` will have a value equal
    /// to the sum of the two tokens (`dst_coin` and `source_coin`).
    public fun merge<CoinType>(dst_coin: &mut Coin<CoinType>, source_coin: Coin<CoinType>) {
        spec {
            assume dst_coin.value + source_coin.value <= MAX_U64;
        };
        spec {
            update supply<CoinType> = supply<CoinType> - source_coin.value;
        };
        let Coin { value } = source_coin;
        spec {
            update supply<CoinType> = supply<CoinType> + value;
        };
        dst_coin.value = dst_coin.value + value;
    }

    /// Mint new `Coin` with capability.
    /// The capability `_cap` should be passed as reference to `MintCapability<CoinType>`.
    /// Returns minted `Coin`.
    public fun mint<CoinType>(
        amount: u64,
        _cap: &MintCapability<CoinType>,
    ): Coin<CoinType> acquires CoinInfo {
        mint_internal<CoinType>(amount)
    }

    #[deprecated]
    public fun register<CoinType>(_account: &signer) {
        // Noop since fungible asset does not need register.
    }

    /// Transfers `amount` of coins `CoinType` from `from` to `to`.
    public entry fun transfer<CoinType>(
        from: &signer,
        to: address,
        amount: u64,
    ) acquires CoinStore, CoinConversionMap, CoinInfo {
        let coin = withdraw<CoinType>(from, amount);
        deposit(to, coin);
    }

    /// Returns the `value` passed in `coin`.
    public fun value<CoinType>(coin: &Coin<CoinType>): u64 {
        coin.value
    }

    /// Withdraw specified `amount` of coin `CoinType` from the signing account.
    public fun withdraw<CoinType>(
        account: &signer,
        amount: u64,
    ): Coin<CoinType> acquires CoinStore, CoinConversionMap, CoinInfo {
        let account_addr = signer::address_of(account);
        maybe_convert_to_fungible_store<CoinType>(account_addr);
        let fa = primary_fungible_store::withdraw(account, paired_metadata<CoinType>(), amount);
        fungible_asset_to_coin<CoinType>(fa)
    }

    /// Create a new `Coin<CoinType>` with a value of `0`.
    public fun zero<CoinType>(): Coin<CoinType> {
        spec {
            update supply<CoinType> = supply<CoinType> + 0;
        };
        Coin<CoinType> {
            value: 0
        }
    }

    /// Destroy a freeze capability. Freeze capability is dangerous and therefore should be destroyed if not used.
    public fun destroy_freeze_cap<CoinType>(freeze_cap: FreezeCapability<CoinType>) {
        let FreezeCapability<CoinType> {} = freeze_cap;
    }

    /// Destroy a mint capability.
    public fun destroy_mint_cap<CoinType>(mint_cap: MintCapability<CoinType>) {
        let MintCapability<CoinType> {} = mint_cap;
    }

    /// Destroy a burn capability.
    public fun destroy_burn_cap<CoinType>(burn_cap: BurnCapability<CoinType>) {
        let BurnCapability<CoinType> {} = burn_cap;
    }

    fun mint_internal<CoinType>(amount: u64): Coin<CoinType> acquires CoinInfo {
        if (amount == 0) {
            return Coin<CoinType> {
                value: 0
            }
        };

        let maybe_supply = &mut borrow_global_mut<CoinInfo<CoinType>>(coin_address<CoinType>()).supply;
        if (option::is_some(maybe_supply)) {
            let supply = option::borrow_mut(maybe_supply);
            spec {
                use aptos_framework::optional_aggregator;
                use aptos_framework::aggregator;
                assume optional_aggregator::is_parallelizable(supply) ==> (aggregator::spec_aggregator_get_val(
                    option::borrow(supply.aggregator)
                )
                    + amount <= aggregator::spec_get_limit(option::borrow(supply.aggregator)));
                assume !optional_aggregator::is_parallelizable(supply) ==>
                    (option::borrow(supply.integer).value + amount <= option::borrow(supply.integer).limit);
            };
            optional_aggregator::add(supply, (amount as u128));
        };
        spec {
            update supply<CoinType> = supply<CoinType> + amount;
        };
        Coin<CoinType> { value: amount }
    }

    inline fun burn_internal<CoinType>(coin: Coin<CoinType>): u64 {
        spec {
            update supply<CoinType> = supply<CoinType> - coin.value;
        };
        let Coin { value: amount } = coin;
        if (amount != 0) {
            let maybe_supply = &mut borrow_global_mut<CoinInfo<CoinType>>(coin_address<CoinType>()).supply;
            if (option::is_some(maybe_supply)) {
                let supply = option::borrow_mut(maybe_supply);
                optional_aggregator::sub(supply, (amount as u128));
            };
        };
        amount
    }

    #[test_only]
    struct FakeMoney {}

    #[test_only]
    struct FakeMoneyCapabilities has key {
        burn_cap: BurnCapability<FakeMoney>,
        freeze_cap: FreezeCapability<FakeMoney>,
        mint_cap: MintCapability<FakeMoney>,
    }

    #[test_only]
    struct FakeMoneyRefs has key {
        mint_ref: MintRef,
        transfer_ref: TransferRef,
        burn_ref: BurnRef,
    }

    #[test_only]
    fun create_coin_store<CoinType>(account: &signer) {
        if (!exists<CoinStore<CoinType>>(signer::address_of(account))) {
            let coin_store = CoinStore<CoinType> {
                coin: Coin { value: 0 },
                frozen: false,
                deposit_events: account::new_event_handle<DepositEvent>(account),
                withdraw_events: account::new_event_handle<WithdrawEvent>(account),
            };
            move_to(account, coin_store);
        }
    }

    #[test_only]
    fun coin_store_exists<CoinType>(account: address): bool {
        exists<CoinStore<CoinType>>(account)
    }

    #[test_only]
    fun initialize_fake_money(
        account: &signer,
        decimals: u8,
        monitor_supply: bool,
    ): (BurnCapability<FakeMoney>, FreezeCapability<FakeMoney>, MintCapability<FakeMoney>) {
        aggregator_factory::initialize_aggregator_factory_for_test(account);
        initialize<FakeMoney>(
            account,
            string::utf8(b"Fake money"),
            string::utf8(b"FMD"),
            decimals,
            monitor_supply
        )
    }

    #[test_only]
    fun initialize_and_register_fake_money(
        account: &signer,
        decimals: u8,
        monitor_supply: bool,
    ): (BurnCapability<FakeMoney>, FreezeCapability<FakeMoney>, MintCapability<FakeMoney>) {
        let (burn_cap, freeze_cap, mint_cap) = initialize_fake_money(
            account,
            decimals,
            monitor_supply
        );
        create_coin_store<FakeMoney>(account);
        (burn_cap, freeze_cap, mint_cap)
    }

    #[test_only]
    public entry fun create_fake_money(
        source: &signer,
        destination: &signer,
        amount: u64
    ) acquires CoinInfo, CoinStore, CoinConversionMap {
        let (burn_cap, freeze_cap, mint_cap) = initialize_and_register_fake_money(source, 18, true);

        create_coin_store<FakeMoney>(destination);
        let coins_minted = mint<FakeMoney>(amount, &mint_cap);
        deposit(signer::address_of(source), coins_minted);
        move_to(source, FakeMoneyCapabilities {
            burn_cap,
            freeze_cap,
            mint_cap,
        });
    }

    #[test(source = @0x1, destination = @0x2)]
    public entry fun end_to_end(
        source: signer,
        destination: signer,
    ) acquires CoinInfo, CoinStore, CoinConversionMap {
        let source_addr = signer::address_of(&source);
        account::create_account_for_test(source_addr);
        let destination_addr = signer::address_of(&destination);
        account::create_account_for_test(destination_addr);

        let name = string::utf8(b"Fake money");
        let symbol = string::utf8(b"FMD");

        aggregator_factory::initialize_aggregator_factory_for_test(&source);
        let (burn_cap, freeze_cap, mint_cap) = initialize<FakeMoney>(
            &source,
            name,
            symbol,
            18,
            true
        );
        create_coin_store<FakeMoney>(&source);
        create_coin_store<FakeMoney>(&destination);
        assert!(*option::borrow(&supply<FakeMoney>()) == 0, 0);

        assert!(name<FakeMoney>() == name, 1);
        assert!(symbol<FakeMoney>() == symbol, 2);
        assert!(decimals<FakeMoney>() == 18, 3);

        let coins_minted = mint<FakeMoney>(100, &mint_cap);
        deposit(source_addr, coins_minted);
        assert!(!coin_store_exists<FakeMoney>(source_addr), 0);
        assert!(coin_store_exists<FakeMoney>(destination_addr), 0);

        transfer<FakeMoney>(&source, destination_addr, 50);
        assert!(!coin_store_exists<FakeMoney>(destination_addr), 0);

        assert!(balance<FakeMoney>(source_addr) == 50, 4);
        assert!(balance<FakeMoney>(destination_addr) == 50, 5);
        assert!(*option::borrow(&supply<FakeMoney>()) == 100, 6);

        let coin = withdraw<FakeMoney>(&source, 10);
        assert!(value(&coin) == 10, 7);
        burn(coin, &burn_cap);
        assert!(*option::borrow(&supply<FakeMoney>()) == 90, 8);

        move_to(&source, FakeMoneyCapabilities {
            burn_cap,
            freeze_cap,
            mint_cap,
        });
    }

    #[test(source = @0x1, destination = @0x2)]
    public entry fun end_to_end_no_supply(
        source: signer,
        destination: signer,
    ) acquires CoinInfo, CoinStore, CoinConversionMap {
        let source_addr = signer::address_of(&source);
        account::create_account_for_test(source_addr);
        let destination_addr = signer::address_of(&destination);
        account::create_account_for_test(destination_addr);

        let (burn_cap, freeze_cap, mint_cap) = initialize_and_register_fake_money(&source, 1, false);

        register<FakeMoney>(&destination);
        assert!(option::is_none(&supply<FakeMoney>()), 0);

        let coins_minted = mint<FakeMoney>(100, &mint_cap);
        deposit<FakeMoney>(source_addr, coins_minted);
        transfer<FakeMoney>(&source, destination_addr, 50);

        assert!(balance<FakeMoney>(source_addr) == 50, 1);
        assert!(balance<FakeMoney>(destination_addr) == 50, 2);
        assert!(option::is_none(&supply<FakeMoney>()), 3);

        let coin = withdraw<FakeMoney>(&source, 10);
        burn(coin, &burn_cap);
        assert!(option::is_none(&supply<FakeMoney>()), 4);

        move_to(&source, FakeMoneyCapabilities {
            burn_cap,
            freeze_cap,
            mint_cap,
        });
    }

    #[test(source = @0x2, framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x10001, location = Self)]
    public fun fail_initialize(source: signer, framework: signer) {
        aggregator_factory::initialize_aggregator_factory_for_test(&framework);
        let (burn_cap, freeze_cap, mint_cap) = initialize<FakeMoney>(
            &source,
            string::utf8(b"Fake money"),
            string::utf8(b"FMD"),
            1,
            true,
        );

        move_to(&source, FakeMoneyCapabilities {
            burn_cap,
            freeze_cap,
            mint_cap,
        });
    }

    #[test(source = @0x1, destination = @0x2)]
    public entry fun transfer_to_destination_without_coin_store(
        source: signer,
        destination: signer,
    ) acquires CoinInfo, CoinStore, CoinConversionMap {
        let source_addr = signer::address_of(&source);
        account::create_account_for_test(source_addr);
        let destination_addr = signer::address_of(&destination);
        account::create_account_for_test(destination_addr);

        let (burn_cap, freeze_cap, mint_cap) = initialize_and_register_fake_money(&source, 1, true);
        assert!(*option::borrow(&supply<FakeMoney>()) == 0, 0);

        let coins_minted = mint<FakeMoney>(100, &mint_cap);
        deposit(source_addr, coins_minted);
        assert!(!coin_store_exists<FakeMoney>(source_addr), 0);
        transfer<FakeMoney>(&source, destination_addr, 50);
        assert!(balance<FakeMoney>(destination_addr) == 50, 2);
        assert!(!coin_store_exists<FakeMoney>(destination_addr), 0);

        move_to(&source, FakeMoneyCapabilities {
            burn_cap,
            freeze_cap,
            mint_cap,
        });
    }

    #[test(source = @0x1)]
    public entry fun test_burn_from_with_capability(
        source: signer,
    ) acquires CoinInfo, CoinStore, CoinConversionMap {
        let source_addr = signer::address_of(&source);
        account::create_account_for_test(source_addr);
        let (burn_cap, freeze_cap, mint_cap) = initialize_and_register_fake_money(&source, 1, true);

        let coins_minted = mint<FakeMoney>(100, &mint_cap);
        deposit(source_addr, coins_minted);
        assert!(!coin_store_exists<FakeMoney>(source_addr), 0);
        assert!(balance<FakeMoney>(source_addr) == 100, 0);
        assert!(*option::borrow(&supply<FakeMoney>()) == 100, 1);

        burn_from<FakeMoney>(source_addr, 10, &burn_cap);
        assert!(balance<FakeMoney>(source_addr) == 90, 2);
        assert!(*option::borrow(&supply<FakeMoney>()) == 90, 3);

        move_to(&source, FakeMoneyCapabilities {
            burn_cap,
            freeze_cap,
            mint_cap,
        });
    }

    #[test(source = @0x1)]
    #[expected_failure(abort_code = 0x10007, location = Self)]
    public fun test_destroy_non_zero(
        source: signer,
    ) acquires CoinInfo {
        account::create_account_for_test(signer::address_of(&source));
        let (burn_cap, freeze_cap, mint_cap) = initialize_and_register_fake_money(&source, 1, true);
        let coins_minted = mint<FakeMoney>(100, &mint_cap);
        destroy_zero(coins_minted);

        move_to(&source, FakeMoneyCapabilities {
            burn_cap,
            freeze_cap,
            mint_cap,
        });
    }

    #[test(source = @0x1)]
    public entry fun test_extract(
        source: signer,
    ) acquires CoinInfo, CoinStore, CoinConversionMap {
        let source_addr = signer::address_of(&source);
        account::create_account_for_test(source_addr);
        let (burn_cap, freeze_cap, mint_cap) = initialize_and_register_fake_money(&source, 1, true);

        let coins_minted = mint<FakeMoney>(100, &mint_cap);

        let extracted = extract(&mut coins_minted, 25);
        assert!(value(&coins_minted) == 75, 0);
        assert!(value(&extracted) == 25, 1);

        deposit(source_addr, coins_minted);
        deposit(source_addr, extracted);

        assert!(balance<FakeMoney>(source_addr) == 100, 2);

        move_to(&source, FakeMoneyCapabilities {
            burn_cap,
            freeze_cap,
            mint_cap,
        });
    }

    #[test(source = @0x1)]
    public fun test_is_coin_initialized(source: signer) {
        assert!(!is_coin_initialized<FakeMoney>(), 0);

        let (burn_cap, freeze_cap, mint_cap) = initialize_fake_money(&source, 1, true);
        assert!(is_coin_initialized<FakeMoney>(), 1);

        move_to(&source, FakeMoneyCapabilities {
            burn_cap,
            freeze_cap,
            mint_cap,
        });
    }

    #[test(account = @0x1)]
    public fun test_is_coin_store_frozen(account: signer) acquires CoinStore, CoinConversionMap, CoinInfo {
        let account_addr = signer::address_of(&account);
        account::create_account_for_test(account_addr);
        let (burn_cap, freeze_cap, mint_cap) = initialize_and_register_fake_money(&account, 18, true);
        assert!(coin_store_exists<FakeMoney>(account_addr), 1);
        assert!(!is_coin_store_frozen<FakeMoney>(account_addr), 1);
        assert!(!coin_store_exists<FakeMoney>(account_addr), 1);

        // freeze account
        freeze_coin_store(account_addr, &freeze_cap);
        assert!(is_coin_store_frozen<FakeMoney>(account_addr), 1);
        assert!(primary_fungible_store::is_frozen(account_addr, paired_metadata<FakeMoney>()), 1);

        // unfreeze account
        unfreeze_coin_store(account_addr, &freeze_cap);
        assert!(!is_coin_store_frozen<FakeMoney>(account_addr), 1);
        assert!(!primary_fungible_store::is_frozen(account_addr, paired_metadata<FakeMoney>()), 1);

        move_to(&account, FakeMoneyCapabilities {
            burn_cap,
            freeze_cap,
            mint_cap,
        });
    }

    #[test]
    fun test_zero() {
        let zero = zero<FakeMoney>();
        assert!(value(&zero) == 0, 1);
        destroy_zero(zero);
    }

    #[test(account = @0x1)]
    public entry fun burn_frozen(account: signer) acquires CoinInfo, CoinStore, CoinConversionMap {
        let account_addr = signer::address_of(&account);
        account::create_account_for_test(account_addr);
        let (burn_cap, freeze_cap, mint_cap) = initialize_and_register_fake_money(&account, 18, true);

        let coins_minted = mint<FakeMoney>(100, &mint_cap);
        deposit(account_addr, coins_minted);

        freeze_coin_store(account_addr, &freeze_cap);
        burn_from(account_addr, 90, &burn_cap);
        assert!(primary_fungible_store::is_frozen(account_addr, paired_metadata<FakeMoney>()), 1);
        assert!(primary_fungible_store::balance(account_addr, paired_metadata<FakeMoney>()) == 10, 1);

        move_to(&account, FakeMoneyCapabilities {
            burn_cap,
            freeze_cap,
            mint_cap,
        });
    }

    #[test(account = @0x1)]
    #[expected_failure(abort_code = 0x50003, location = aptos_framework::fungible_asset)]
    public entry fun withdraw_frozen(account: signer) acquires CoinInfo, CoinStore, CoinConversionMap {
        let account_addr = signer::address_of(&account);
        account::create_account_for_test(account_addr);
        let (burn_cap, freeze_cap, mint_cap) = initialize_and_register_fake_money(&account, 18, true);
        let coins_minted = mint<FakeMoney>(100, &mint_cap);
        deposit(account_addr, coins_minted);

        freeze_coin_store(account_addr, &freeze_cap);
        let coin = withdraw<FakeMoney>(&account, 90);
        assert!(!primary_fungible_store::is_frozen(account_addr, paired_metadata<FakeMoney>()), 1);
        assert!(primary_fungible_store::balance(account_addr, paired_metadata<FakeMoney>()) == 10, 1);
        burn(coin, &burn_cap);

        move_to(&account, FakeMoneyCapabilities {
            burn_cap,
            freeze_cap,
            mint_cap,
        });
    }

    #[test(account = @0x1)]
    #[expected_failure(abort_code = 0x50003, location = aptos_framework::fungible_asset)]
    public entry fun deposit_frozen(account: signer) acquires CoinInfo, CoinStore, CoinConversionMap {
        let account_addr = signer::address_of(&account);
        account::create_account_for_test(account_addr);
        let (burn_cap, freeze_cap, mint_cap) = initialize_and_register_fake_money(&account, 18, true);

        let coins_minted = mint<FakeMoney>(100, &mint_cap);
        freeze_coin_store(account_addr, &freeze_cap);
        deposit(account_addr, coins_minted);

        move_to(&account, FakeMoneyCapabilities {
            burn_cap,
            freeze_cap,
            mint_cap,
        });
    }

    #[test(account = @0x1)]
    public entry fun deposit_widthdraw_unfrozen(account: signer) acquires CoinInfo, CoinStore, CoinConversionMap {
        let account_addr = signer::address_of(&account);
        account::create_account_for_test(account_addr);
        let (burn_cap, freeze_cap, mint_cap) = initialize_and_register_fake_money(&account, 18, true);

        let coins_minted = mint<FakeMoney>(100, &mint_cap);
        freeze_coin_store(account_addr, &freeze_cap);
        unfreeze_coin_store(account_addr, &freeze_cap);
        deposit(account_addr, coins_minted);

        freeze_coin_store(account_addr, &freeze_cap);
        unfreeze_coin_store(account_addr, &freeze_cap);
        let coin = withdraw<FakeMoney>(&account, 10);
        burn(coin, &burn_cap);

        move_to(&account, FakeMoneyCapabilities {
            burn_cap,
            freeze_cap,
            mint_cap,
        });
    }

    #[test_only]
    fun initialize_with_aggregator(account: &signer) {
        let (burn_cap, freeze_cap, mint_cap) = initialize_with_parallelizable_supply<FakeMoney>(
            account,
            string::utf8(b"Fake money"),
            string::utf8(b"FMD"),
            1,
            true
        );
        move_to(account, FakeMoneyCapabilities {
            burn_cap,
            freeze_cap,
            mint_cap,
        });
    }

    #[test_only]
    fun initialize_with_integer(account: &signer) {
        let (burn_cap, freeze_cap, mint_cap) = initialize<FakeMoney>(
            account,
            string::utf8(b"Fake money"),
            string::utf8(b"FMD"),
            1,
            true
        );
        move_to(account, FakeMoneyCapabilities {
            burn_cap,
            freeze_cap,
            mint_cap,
        });
    }


    #[test(framework = @aptos_framework, other = @0x123)]
    #[expected_failure(abort_code = 0x50003, location = aptos_framework::system_addresses)]
    fun test_supply_initialize_fails(framework: signer, other: signer) {
        aggregator_factory::initialize_aggregator_factory_for_test(&framework);
        initialize_with_aggregator(&other);
    }

    #[test(framework = @aptos_framework)]
    fun test_supply_initialize(framework: signer) acquires CoinInfo {
        aggregator_factory::initialize_aggregator_factory_for_test(&framework);
        initialize_with_aggregator(&framework);

        let maybe_supply = &mut borrow_global_mut<CoinInfo<FakeMoney>>(coin_address<FakeMoney>()).supply;
        let supply = option::borrow_mut(maybe_supply);

        // Supply should be parallelizable.
        assert!(optional_aggregator::is_parallelizable(supply), 0);

        optional_aggregator::add(supply, 100);
        optional_aggregator::sub(supply, 50);
        optional_aggregator::add(supply, 950);
        assert!(optional_aggregator::read(supply) == 1000, 0);
    }

    #[test(framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x20001, location = aptos_framework::aggregator)]
    fun test_supply_overflow(framework: signer) acquires CoinInfo {
        aggregator_factory::initialize_aggregator_factory_for_test(&framework);
        initialize_with_aggregator(&framework);

        let maybe_supply = &mut borrow_global_mut<CoinInfo<FakeMoney>>(coin_address<FakeMoney>()).supply;
        let supply = option::borrow_mut(maybe_supply);

        optional_aggregator::add(supply, MAX_U128);
        optional_aggregator::add(supply, 1);
        optional_aggregator::sub(supply, 1);
    }

    #[test(framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x5000B, location = aptos_framework::coin)]
    fun test_supply_upgrade_fails(framework: signer) acquires CoinInfo, SupplyConfig {
        initialize_supply_config(&framework);
        aggregator_factory::initialize_aggregator_factory_for_test(&framework);
        initialize_with_integer(&framework);

        let maybe_supply = &mut borrow_global_mut<CoinInfo<FakeMoney>>(coin_address<FakeMoney>()).supply;
        let supply = option::borrow_mut(maybe_supply);

        // Supply should be non-parallelizable.
        assert!(!optional_aggregator::is_parallelizable(supply), 0);

        optional_aggregator::add(supply, 100);
        optional_aggregator::sub(supply, 50);
        optional_aggregator::add(supply, 950);
        assert!(optional_aggregator::read(supply) == 1000, 0);

        upgrade_supply<FakeMoney>(&framework);
    }

    #[test(framework = @aptos_framework)]
    fun test_supply_upgrade(framework: signer) acquires CoinInfo, SupplyConfig {
        initialize_supply_config(&framework);
        aggregator_factory::initialize_aggregator_factory_for_test(&framework);
        initialize_with_integer(&framework);

        // Ensure we have a non-parellelizable non-zero supply.
        let maybe_supply = &mut borrow_global_mut<CoinInfo<FakeMoney>>(coin_address<FakeMoney>()).supply;
        let supply = option::borrow_mut(maybe_supply);
        assert!(!optional_aggregator::is_parallelizable(supply), 0);
        optional_aggregator::add(supply, 100);

        // Upgrade.
        allow_supply_upgrades(&framework, true);
        upgrade_supply<FakeMoney>(&framework);

        // Check supply again.
        let maybe_supply = &mut borrow_global_mut<CoinInfo<FakeMoney>>(coin_address<FakeMoney>()).supply;
        let supply = option::borrow_mut(maybe_supply);
        assert!(optional_aggregator::is_parallelizable(supply), 0);
        assert!(optional_aggregator::read(supply) == 100, 0);
    }

    #[test_only]
    fun destroy_aggregatable_coin_for_test<CoinType>(aggregatable_coin: AggregatableCoin<CoinType>) {
        let AggregatableCoin { value } = aggregatable_coin;
        aggregator::destroy(value);
    }

    #[test(framework = @aptos_framework)]
    public entry fun test_collect_from_and_drain(
        framework: signer,
    ) acquires CoinInfo, CoinStore, CoinConversionMap {
        let framework_addr = signer::address_of(&framework);
        account::create_account_for_test(framework_addr);
        let (burn_cap, freeze_cap, mint_cap) = initialize_and_register_fake_money(&framework, 1, true);

        let coins_minted = mint<FakeMoney>(100, &mint_cap);
        deposit(framework_addr, coins_minted);
        assert!(balance<FakeMoney>(framework_addr) == 100, 0);
        assert!(*option::borrow(&supply<FakeMoney>()) == 100, 0);

        let aggregatable_coin = initialize_aggregatable_coin<FakeMoney>(&framework);
        collect_into_aggregatable_coin<FakeMoney>(framework_addr, 10, &mut aggregatable_coin);

        // Check that aggregatable coin has the right amount.
        let collected_coin = drain_aggregatable_coin(&mut aggregatable_coin);
        assert!(is_aggregatable_coin_zero(&aggregatable_coin), 0);
        assert!(value(&collected_coin) == 10, 0);

        // Supply of coins should be unchanged, but the balance on the account should decrease.
        assert!(balance<FakeMoney>(framework_addr) == 90, 0);
        assert!(*option::borrow(&supply<FakeMoney>()) == 100, 0);

        burn(collected_coin, &burn_cap);
        destroy_aggregatable_coin_for_test(aggregatable_coin);
        move_to(&framework, FakeMoneyCapabilities {
            burn_cap,
            freeze_cap,
            mint_cap,
        });
    }

    #[test_only]
    fun deposit_to_coin_store<CoinType>(account_addr: address, coin: Coin<CoinType>) acquires CoinStore {
        assert!(
            coin_store_exists<CoinType>(account_addr),
            error::not_found(ECOIN_STORE_NOT_PUBLISHED),
        );

        let coin_store = borrow_global_mut<CoinStore<CoinType>>(account_addr);
        assert!(
            !coin_store.frozen,
            error::permission_denied(EFROZEN),
        );

        event::emit_event<DepositEvent>(
            &mut coin_store.deposit_events,
            DepositEvent { amount: coin.value },
        );

        merge(&mut coin_store.coin, coin);
    }

    #[test(account = @aptos_framework)]
    fun test_conversion_basic(account: &signer) acquires CoinConversionMap, CoinInfo, CoinStore {
        let account_addr = signer::address_of(account);
        account::create_account_for_test(account_addr);
        let (burn_cap, freeze_cap, mint_cap) = initialize_and_register_fake_money(account, 1, true);

        assert!(fungible_asset::name(paired_metadata<FakeMoney>()) == name<FakeMoney>(), 0);
        assert!(fungible_asset::symbol(paired_metadata<FakeMoney>()) == symbol<FakeMoney>(), 0);
        assert!(fungible_asset::decimals(paired_metadata<FakeMoney>()) == decimals<FakeMoney>(), 0);
        let minted_coin = mint(100, &mint_cap);
        let converted_fa = coin_to_fungible_asset(minted_coin);
        let minted_fa = fungible_asset::mint(&paired_mint_ref(&mint_cap), 100);
        assert!(&converted_fa == &minted_fa, 0);

        let coin = fungible_asset_to_coin<FakeMoney>(converted_fa);
        assert!(value(&coin) == 100, 0);

        deposit_to_coin_store(account_addr, coin);
        assert!(coin_store_exists<FakeMoney>(account_addr), 0);
        primary_fungible_store::deposit(account_addr, minted_fa);
        assert!(balance<FakeMoney>(account_addr) == 200, 0);

        let withdrawn_coin = withdraw<FakeMoney>(account, 1);
        assert!(!coin_store_exists<FakeMoney>(account_addr), 0);
        assert!(balance<FakeMoney>(account_addr) == 199, 0);
        assert!(primary_fungible_store::balance(account_addr, paired_metadata<FakeMoney>()) == 199, 0);
        burn(withdrawn_coin, &burn_cap);
        move_to(account, FakeMoneyCapabilities {
            burn_cap,
            freeze_cap,
            mint_cap,
        });
    }

    #[test(account = @aptos_framework)]
    fun test_deposit(
        account: &signer,
    ) acquires CoinConversionMap, CoinInfo, CoinStore {
        let account_addr = signer::address_of(account);
        account::create_account_for_test(account_addr);
        let (burn_cap, freeze_cap, mint_cap) = initialize_and_register_fake_money(account, 1, true);
        let coin = mint<FakeMoney>(100, &mint_cap);
        deposit_to_coin_store(account_addr, coin);
        assert!(coin_store_exists<FakeMoney>(account_addr), 0);
        assert!(primary_fungible_store::balance(account_addr, paired_metadata<FakeMoney>()) == 0, 0);
        assert!(balance<FakeMoney>(account_addr) == 100, 0);
        assert!(primary_fungible_store::balance(account_addr, paired_metadata<FakeMoney>()) == 100, 0);

        let coin = mint<FakeMoney>(100, &mint_cap);
        deposit(account_addr, coin);
        assert!(!coin_store_exists<FakeMoney>(account_addr), 0);
        assert!(balance<FakeMoney>(account_addr) == 200, 0);
        assert!(primary_fungible_store::balance(account_addr, paired_metadata<FakeMoney>()) == 200, 0);

        move_to(account, FakeMoneyCapabilities {
            burn_cap,
            freeze_cap,
            mint_cap,
        });
    }

    #[test(account = @aptos_framework)]
    fun test_withdraw(
        account: &signer,
    ) acquires CoinConversionMap, CoinInfo, CoinStore {
        let account_addr = signer::address_of(account);
        account::create_account_for_test(account_addr);
        let (burn_cap, freeze_cap, mint_cap) = initialize_and_register_fake_money(account, 1, true);
        let coin = mint<FakeMoney>(100, &mint_cap);
        deposit_to_coin_store(account_addr, coin);
        assert!(coin_store_exists<FakeMoney>(account_addr), 0);
        assert!(primary_fungible_store::balance(account_addr, paired_metadata<FakeMoney>()) == 0, 0);
        assert!(balance<FakeMoney>(account_addr) == 100, 0);
        assert!(primary_fungible_store::balance(account_addr, paired_metadata<FakeMoney>()) == 100, 0);

        let coin = withdraw<FakeMoney>(account, 50);
        assert!(!coin_store_exists<FakeMoney>(account_addr), 0);
        assert!(balance<FakeMoney>(account_addr) == 50, 0);
        assert!(primary_fungible_store::balance(account_addr, paired_metadata<FakeMoney>()) == 50, 0);
        burn(coin, &burn_cap);

        move_to(account, FakeMoneyCapabilities {
            burn_cap,
            freeze_cap,
            mint_cap,
        });
    }

    #[test(account = @aptos_framework)]
    fun test_supply(
        account: &signer,
    ) acquires CoinConversionMap, CoinInfo {
        account::create_account_for_test(signer::address_of(account));
        let (burn_cap, freeze_cap, mint_cap) = initialize_and_register_fake_money(account, 1, true);
        let coin = mint<FakeMoney>(100, &mint_cap);
        let fungible_asset = fungible_asset::mint(&paired_mint_ref(&mint_cap), 50);
        assert!(supply<FakeMoney>() == option::some(150), 0);
        assert!(coin_supply<FakeMoney>() == option::some(100), 0);
        assert!(fungible_asset::supply(paired_metadata<FakeMoney>()) == option::some(50), 0);
        let fa_from_coin = coin_to_fungible_asset(coin);
        assert!(supply<FakeMoney>() == option::some(150), 0);
        assert!(coin_supply<FakeMoney>() == option::some(0), 0);
        assert!(fungible_asset::supply(paired_metadata<FakeMoney>()) == option::some(150), 0);

        let coin_from_fa = fungible_asset_to_coin<FakeMoney>(fungible_asset);
        assert!(supply<FakeMoney>() == option::some(150), 0);
        assert!(coin_supply<FakeMoney>() == option::some(50), 0);
        assert!(fungible_asset::supply(paired_metadata<FakeMoney>()) == option::some(100), 0);
        burn(coin_from_fa, &burn_cap);
        fungible_asset::burn(&paired_burn_ref(&burn_cap), fa_from_coin);
        assert!(supply<FakeMoney>() == option::some(0), 0);
        move_to(account, FakeMoneyCapabilities {
            burn_cap,
            freeze_cap,
            mint_cap,
        });
    }
}
