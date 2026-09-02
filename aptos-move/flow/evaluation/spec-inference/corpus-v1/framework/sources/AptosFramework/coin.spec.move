spec aptos_framework::coin {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: Only the owner of a coin may mint, burn or freeze coins.
    /// Criticality: Critical
    /// Implementation: Acquiring capabilities for a particular CoinType may only occur if the caller has a signer for
    /// the module declaring that type. The initialize function returns these capabilities to the caller.
    /// Enforcement: Formally Verified via [high-level-req-1.1](upgrade_supply) and [high-level-req-1.2](initialize).
    ///
    /// No.: 2
    /// Requirement: Each coin may only be created exactly once.
    /// Criticality: Medium
    /// Implementation: The initialization function may only be called once.
    /// Enforcement: Formally Verified via [high-level-req-2](initialize).
    ///
    /// No.: 3
    /// Requirement: The merging of coins may only be done on coins of the same type.
    /// Criticality: Critical
    /// Implementation: The merge function is limited to merging coins of the same type only.
    /// Enforcement: Formally Verified via [high-level-req-3](merge).
    ///
    /// No.: 4
    /// Requirement: The supply of a coin is only affected by burn and mint operations.
    /// Criticality: High
    /// Implementation: Only mint and burn operations on a coin alter the total supply of coins.
    /// Enforcement: Formally Verified via [high-level-req-4](TotalSupplyNoChange).
    ///
    /// No.: 5
    /// Requirement: Users may register an account for a coin multiple times idempotently.
    /// Criticality: Medium
    /// Implementation: The register function should work idempotently. Importantly, it should not abort if the coin is already registered.
    /// Enforcement: Formally verified via aborts_if on [high-level-req-5](register).
    ///
    /// No.: 6
    /// Requirement: Coin operations should fail if the user has not registered for the coin.
    /// Criticality: Medium
    /// Implementation: Coin operations may succeed only on valid user coin registration.
    /// Enforcement: Formally Verified via [high-level-req-6.1](balance), [high-level-req-6.2](burn_from), [high-level-req-6.3](freeze), [high-level-req-6.4](unfreeze), [high-level-req-6.5](transfer) and [high-level-req-6.6](withdraw).
    ///
    /// No.: 7
    /// Requirement: It should always be possible to (1) determine if a coin exists, and (2) determine if a user registered
    /// an account with a particular coin. If a coin exists, it should always be possible to request the following
    /// information of the coin: (1) Name, (2) Symbol, and (3) Supply.
    /// Criticality: Low
    /// Implementation: The following functions should never abort: (1) is_coin_initialized, and (2) is_account_registered. The following functions should not abort if the coin exists: (1) name, (2) symbol, and (3) supply.
    /// Enforcement: Formally Verified in corresponding functions: [high-level-req-7.1](is_coin_initialized), [high-level-req-7.2](is_account_registered), [high-level-req-7.3](name), [high-level-req-7.4](symbol) and [high-level-req-7.5](supply).
    ///
    /// No.: 8
    /// Requirement: Coin operations should fail if the user's CoinStore is frozen.
    /// Criticality: Medium
    /// Implementation: If the CoinStore of an address is frozen, coin operations are disallowed.
    /// Enforcement: Formally Verified via [high-level-req-8.1](withdraw), [high-level-req-8.2](transfer) and [high-level-req-8.3](deposit).
    ///
    /// No.: 9
    /// Requirement: Utilizing AggregatableCoins does not violate other critical invariants, such as (4).
    /// Criticality: High
    /// Implementation: Utilizing AggregatableCoin does not change the real-supply of any token.
    /// Enforcement: Formally Verified via [high-level-req-9](TotalSupplyNoChange).
    /// </high-level-req>
    ///
    spec module {
        pragma verify = true;
        pragma aborts_if_is_partial;
        global supply<CoinType>: num;
        global aggregate_supply<CoinType>: num;
        // TODO: `migrate_coin_store_to_fungible_store` migrates in an inline `for_each`
        // loop; the schema needs a loop invariant that cannot be attached there.
        apply TotalSupplyTracked<CoinType> to *<CoinType> except initialize, initialize_internal, initialize_with_parallelizable_supply, migrate_coin_store_to_fungible_store;
        // TODO(fa_migration)
        // apply TotalSupplyNoChange<CoinType> to *<CoinType> except mint,
        //     burn, burn_from, initialize, initialize_internal, initialize_with_parallelizable_supply;
    }

    spec fun spec_fun_supply_tracked<CoinType>(
        val: u64, supply: Option<OptionalAggregator>
    ): bool {
        option::is_some(supply) ==>
            val
                == optional_aggregator::optional_aggregator_value(
                    option::borrow(supply)
                )
    }

    spec schema TotalSupplyTracked<CoinType> {
        ensures old(
            spec_fun_supply_tracked<CoinType>(
                supply<CoinType> + aggregate_supply<CoinType>,
                global<CoinInfo<CoinType>>(type_info::type_of<CoinType>().account_address)
                .supply
            )
        ) ==>
            spec_fun_supply_tracked<CoinType>(
                supply<CoinType> + aggregate_supply<CoinType>,
                global<CoinInfo<CoinType>>(type_info::type_of<CoinType>().account_address)
                .supply
            );
    }

    spec fun spec_fun_supply_no_change<CoinType>(
        old_supply: Option<OptionalAggregator>, supply: Option<OptionalAggregator>
    ): bool {
        option::is_some(old_supply) ==>
            optional_aggregator::optional_aggregator_value(option::borrow(old_supply))
                == optional_aggregator::optional_aggregator_value(
                    option::borrow(supply)
                )
    }

    spec schema TotalSupplyNoChange<CoinType> {
        let old_supply = global<CoinInfo<CoinType>>(
            type_info::type_of<CoinType>().account_address
        ).supply;
        let post supply = global<CoinInfo<CoinType>>(
            type_info::type_of<CoinType>().account_address
        ).supply;
        ensures spec_fun_supply_no_change<CoinType>(old_supply, supply);
    }

    spec AggregatableCoin {
        use aptos_framework::aggregator;
        invariant aggregator::spec_get_limit(value) == MAX_U64;
    }

    spec mint {
        let addr = type_info::type_of<CoinType>().account_address;
        modifies global<CoinInfo<CoinType>>(addr);
    }

    spec mint_internal {
        pragma opaque = true;
        let addr = type_info::type_of<CoinType>().account_address;
        modifies global<CoinInfo<CoinType>>(addr);
        aborts_if (amount != 0) && !exists<CoinInfo<CoinType>>(addr);
        ensures supply<CoinType> == old(supply<CoinType>) + amount;
        ensures result.value == amount;
    }

    /// Get address by reflection.
    spec coin_address<CoinType>(): address {
        pragma opaque;
        pragma aborts_if_is_partial = false;
        aborts_if !type_info::spec_is_struct<CoinType>();
        ensures result == type_info::type_of<CoinType>().account_address;
    }

    /// Can only be updated by `@aptos_framework`.
    spec allow_supply_upgrades(_aptos_framework: &signer, _allowed: bool) {
        aborts_if true;
    }

    spec balance<CoinType>(owner: address): u64 {
        // TODO(fa_migration)
        pragma verify = false;
        aborts_if !exists<CoinStore<CoinType>>(owner);
        ensures result == global<CoinStore<CoinType>>(owner).coin.value;
    }

    spec is_coin_initialized<CoinType>(): bool {
        pragma opaque;
        pragma aborts_if_is_partial = false;
        /// [high-level-req-7.1]
        aborts_if !type_info::spec_is_struct<CoinType>();
        ensures result
            == exists<CoinInfo<CoinType>>(type_info::type_of<CoinType>().account_address);
    }

    spec is_account_registered<CoinType>(_account_addr: address): bool {
        pragma opaque;
        pragma aborts_if_is_partial = false;
        aborts_if !type_info::spec_is_struct<CoinType>();
        aborts_if !exists<CoinInfo<CoinType>>(
            type_info::type_of<CoinType>().account_address
        );
        ensures result == true;
    }

    spec fun get_coin_supply_opt<CoinType>(): Option<OptionalAggregator> {
        global<CoinInfo<CoinType>>(type_info::type_of<CoinType>().account_address).supply
    }

    spec fun spec_paired_metadata<CoinType>(): Option<Object<Metadata>> {
        if (exists<CoinConversionMap>(@aptos_framework)) {
            let map =
                global<CoinConversionMap>(@aptos_framework).coin_to_fungible_asset_map;
            if (table::spec_contains(map, type_info::type_of<CoinType>())) {
                let metadata = table::spec_get(map, type_info::type_of<CoinType>());
                option::spec_some(metadata)
            } else {
                option::spec_none()
            }
        } else {
            option::spec_none()
        }
    }

    /// Address at which a pairing either already lives or would be created.
    /// The APT pairing is always present before the `allow_apt_creation = false`
    /// path can succeed; for every other type this is the named-object address.
    spec fun spec_paired_metadata_address<CoinType>(): address {
        let metadata = spec_paired_metadata<CoinType>();
        if (option::is_some(metadata)) {
            object::object_address(option::destroy_some(metadata))
        } else {
            object::spec_create_object_address(
                @aptos_fungible_asset, type_info::type_name<CoinType>().bytes
            )
        }
    }

    spec schema CoinSubAbortsIf<CoinType> {
        use aptos_framework::optional_aggregator;
        amount: u64;
        let addr = type_info::type_of<CoinType>().account_address;
        let maybe_supply = global<CoinInfo<CoinType>>(addr).supply;
        include (option::is_some(maybe_supply)) ==>
            optional_aggregator::SubAbortsIf {
                optional_aggregator: option::borrow(maybe_supply),
                value: amount
            };
    }

    spec schema CoinAddAbortsIf<CoinType> {
        use aptos_framework::optional_aggregator;
        amount: u64;
        let addr = type_info::type_of<CoinType>().account_address;
        let maybe_supply = global<CoinInfo<CoinType>>(addr).supply;
        include (option::is_some(maybe_supply)) ==>
            optional_aggregator::AddAbortsIf {
                optional_aggregator: option::borrow(maybe_supply),
                value: amount
            };
    }

    spec schema AbortsIfNotExistCoinInfo<CoinType> {
        let addr = type_info::type_of<CoinType>().account_address;
        aborts_if !exists<CoinInfo<CoinType>>(addr);
    }

    spec name<CoinType>(): string::String {
        pragma opaque = true;
        pragma aborts_if_is_partial = false;
        /// [high-level-req-7.3]
        aborts_if !type_info::spec_is_struct<CoinType>();
        include AbortsIfNotExistCoinInfo<CoinType>;
        ensures result
            == global<CoinInfo<CoinType>>(
                type_info::type_of<CoinType>().account_address
            ).name;
    }

    spec symbol<CoinType>(): string::String {
        pragma opaque = true;
        pragma aborts_if_is_partial = false;
        /// [high-level-req-7.4]
        aborts_if !type_info::spec_is_struct<CoinType>();
        include AbortsIfNotExistCoinInfo<CoinType>;
        ensures result
            == global<CoinInfo<CoinType>>(
                type_info::type_of<CoinType>().account_address
            ).symbol;
    }

    spec decimals<CoinType>(): u8 {
        pragma opaque = true;
        pragma aborts_if_is_partial = false;
        aborts_if !type_info::spec_is_struct<CoinType>();
        include AbortsIfNotExistCoinInfo<CoinType>;
        ensures result
            == global<CoinInfo<CoinType>>(
                type_info::type_of<CoinType>().account_address
            ).decimals;
    }

    spec supply<CoinType>(): Option<u128> {
        // TODO(fa_migration)
        pragma verify = false;
    }

    spec coin_supply<CoinType>(): Option<u128> {
        let coin_addr = type_info::type_of<CoinType>().account_address;
        /// [high-level-req-7.5]
        aborts_if !exists<CoinInfo<CoinType>>(coin_addr);
        let maybe_supply = global<CoinInfo<CoinType>>(coin_addr).supply;
        let supply = option::borrow(maybe_supply);
        let value = optional_aggregator::optional_aggregator_value(supply);

        ensures if (option::is_some(maybe_supply)) {
            result == option::spec_some(value)
        } else {
            option::is_none(result)
        };
    }

    spec burn<CoinType>(
        coin: Coin<CoinType>, _cap: &BurnCapability<CoinType>
    ) {
        // TODO(fa_migration)
        pragma verify = false;
        let addr = type_info::type_of<CoinType>().account_address;
        modifies global<CoinInfo<CoinType>>(addr);
        include AbortsIfNotExistCoinInfo<CoinType>;
        aborts_if coin.value == 0;
        include CoinSubAbortsIf<CoinType> { amount: coin.value };
        ensures supply<CoinType> == old(supply<CoinType>) - coin.value;
    }

    spec burn_internal<CoinType>(coin: Coin<CoinType>): u64 {
        use 0x1::optional_aggregator;
        pragma opaque = true, aborts_if_is_partial = false;
        let addr = type_info::type_of<CoinType>().account_address;
        modifies global<CoinInfo<CoinType>>(addr);
        ensures result == coin.value;
        aborts_if coin.value != 0 && aborts_of<coin_address<CoinType>> ();
        aborts_if coin.value != 0
            && !aborts_of<coin_address<CoinType>> ()
            && !exists<CoinInfo<CoinType>>(addr);
        aborts_if coin.value != 0
            && exists<CoinInfo<CoinType>>(addr)
            && option::is_some(global<CoinInfo<CoinType>>(addr).supply)
            && aborts_of<optional_aggregator::sub>(
                option::borrow(global<CoinInfo<CoinType>> (addr).supply), coin.value as u128
            );
    }

    spec burn_from<CoinType>(
        account_addr: address, amount: u64, burn_cap: &BurnCapability<CoinType>
    ) {
        // TODO(fa_migration)
        pragma verify = false;
        let addr = type_info::type_of<CoinType>().account_address;
        let coin_store = global<CoinStore<CoinType>>(account_addr);
        let post post_coin_store = global<CoinStore<CoinType>>(account_addr);

        modifies global<CoinInfo<CoinType>>(addr);
        modifies global<CoinStore<CoinType>>(account_addr);

        /// [high-level-req-6.2]
        aborts_if amount != 0 && !exists<CoinInfo<CoinType>>(addr);
        aborts_if amount != 0 && !exists<CoinStore<CoinType>>(account_addr);
        aborts_if coin_store.coin.value < amount;

        let maybe_supply = global<CoinInfo<CoinType>>(addr).supply;
        let supply_aggr = option::borrow(maybe_supply);
        let value = optional_aggregator::optional_aggregator_value(supply_aggr);

        let post post_maybe_supply = global<CoinInfo<CoinType>>(addr).supply;
        let post post_supply = option::borrow(post_maybe_supply);
        let post post_value = optional_aggregator::optional_aggregator_value(post_supply);

        aborts_if option::is_some(maybe_supply) && value < amount;

        ensures post_coin_store.coin.value == coin_store.coin.value - amount;
        /// [managed_coin::high-level-req-5]
        ensures if (option::is_some(maybe_supply)) {
            post_value == value - amount
        } else {
            option::is_none(post_maybe_supply)
        };
        ensures supply<CoinType> == old(supply<CoinType>) - amount;
    }

    /// `account_addr` is not frozen.
    spec deposit<CoinType>(account_addr: address, coin: Coin<CoinType>) {
        use 0x1::fungible_asset;
        use 0x1::object;
        use 0x1::primary_fungible_store;
        pragma opaque = true, aborts_if_is_partial = false;
        let coin_info_addr = type_info::type_of<CoinType>().account_address;
        let metadata_addr = spec_paired_metadata_address<CoinType>();
        let store_addr = object::spec_create_user_derived_object_address(
            account_addr, metadata_addr
        );
        modifies CoinInfo<CoinType>[coin_info_addr];
        modifies CoinConversionMap[@aptos_framework];
        modifies PairedCoinType[metadata_addr];
        modifies PairedFungibleAssetRefs[metadata_addr];
        modifies object::ObjectCore[metadata_addr];
        modifies fungible_asset::Metadata[metadata_addr];
        modifies fungible_asset::Supply[metadata_addr];
        modifies fungible_asset::ConcurrentSupply[metadata_addr];
        modifies primary_fungible_store::DeriveRefPod[metadata_addr];
        modifies object::ObjectCore[store_addr];
        modifies fungible_asset::FungibleStore[store_addr];
        modifies fungible_asset::ConcurrentFungibleBalance[store_addr];
        modifies object::Untransferable[store_addr];
        ensures {
            let fa = ..S1 |~ result_of<coin_to_fungible_asset<CoinType>> (coin);
            S1.. |~ ensures_of<primary_fungible_store::deposit>(account_addr, fa)
        };
        aborts_if aborts_of<coin_to_fungible_asset<CoinType>> (coin);
        aborts_if {
            let fa = ..S1 |~ result_of<coin_to_fungible_asset<CoinType>> (coin);
            S1 |~ aborts_of<primary_fungible_store::deposit>(account_addr, fa)
        };
    }

    spec coin_to_fungible_asset<CoinType>(coin: Coin<CoinType>): FungibleAsset {
        use 0x1::fungible_asset;
        use 0x1::object;
        use 0x1::primary_fungible_store;
        pragma opaque = true, aborts_if_is_partial = false;
        let addr = type_info::type_of<CoinType>().account_address;
        let metadata_addr = spec_paired_metadata_address<CoinType>();
        modifies CoinInfo<CoinType>[addr];
        modifies CoinConversionMap[@aptos_framework];
        modifies PairedCoinType[metadata_addr];
        modifies PairedFungibleAssetRefs[metadata_addr];
        modifies object::ObjectCore[metadata_addr];
        modifies fungible_asset::Metadata[metadata_addr];
        modifies fungible_asset::Supply[metadata_addr];
        modifies fungible_asset::ConcurrentSupply[metadata_addr];
        modifies primary_fungible_store::DeriveRefPod[metadata_addr];
        ensures {
            let metadata = ..S1 |~ result_of<ensure_paired_metadata<CoinType>> ();
            let amount = S1..S2 |~ result_of<burn_internal<CoinType>> (coin);
            result == (S2.. |~ result_of<fungible_asset::mint_internal>(metadata, amount))
        };
        aborts_if aborts_of<ensure_paired_metadata<CoinType>> ();
        aborts_if {
            let metadata = ..S1 |~ result_of<ensure_paired_metadata<CoinType>> ();
            S1 |~ aborts_of<burn_internal<CoinType>> (coin)
        };
        aborts_if {
            let metadata = ..S1 |~ result_of<ensure_paired_metadata<CoinType>> ();
            let amount = S1..S2 |~ result_of<burn_internal<CoinType>> (coin);
            S2 |~ aborts_of<fungible_asset::mint_internal>(metadata, amount)
        };
    }

    spec fungible_asset_to_coin<CoinType>(
        fungible_asset: FungibleAsset
    ): Coin<CoinType> {
        pragma opaque = true;
        // TODO(fa_migration)
        pragma verify = false;
    }

    spec maybe_convert_to_fungible_store<CoinType>(account: address) {
        // TODO(fa_migration)
        pragma verify = false;
        modifies global<CoinInfo<CoinType>>(account);
        modifies global<CoinStore<CoinType>>(account);
    }

    spec schema DepositAbortsIf<CoinType> {
        account_addr: address;
        let coin_store = global<CoinStore<CoinType>>(account_addr);
        aborts_if !exists<CoinStore<CoinType>>(account_addr);
        aborts_if coin_store.frozen;
    }

    spec deposit_for_gas_fee<CoinType>(
        account_addr: address, coin: Coin<CoinType>
    ) {
        // TODO(fa_migration)
        pragma verify = false;
        modifies global<CoinStore<CoinType>>(account_addr);
        aborts_if !exists<CoinStore<CoinType>>(account_addr);
        ensures global<CoinStore<CoinType>>(account_addr).coin.value
            == old(global<CoinStore<CoinType>>(account_addr)).coin.value + coin.value;
    }

    /// The value of `zero_coin` must be 0.
    spec destroy_zero<CoinType>(zero_coin: Coin<CoinType>) {
        pragma opaque = true;
        pragma aborts_if_is_partial = false;
        aborts_if zero_coin.value > 0;
    }

    spec extract<CoinType>(coin: &mut Coin<CoinType>, amount: u64): Coin<CoinType> {
        pragma opaque = true;
        pragma aborts_if_is_partial = false;
        aborts_if coin.value < amount;
        ensures result.value == amount;
        ensures coin.value == old(coin.value) - amount;
    }

    spec extract_all<CoinType>(coin: &mut Coin<CoinType>): Coin<CoinType> {
        pragma opaque;
        pragma aborts_if_is_partial = false;
        aborts_if false;
        ensures result.value == old(coin).value;
        ensures coin.value == 0;
    }

    spec freeze_coin_store<CoinType>(
        account_addr: address, _freeze_cap: &FreezeCapability<CoinType>
    ) {
        // TODO(fa_migration)
        pragma verify = false;
        // pragma opaque;
        modifies global<CoinStore<CoinType>>(account_addr);
        /// [high-level-req-6.3]
        aborts_if !exists<CoinStore<CoinType>>(account_addr);
        let post coin_store = global<CoinStore<CoinType>>(account_addr);
        ensures coin_store.frozen;
    }

    spec unfreeze_coin_store<CoinType>(
        account_addr: address, _freeze_cap: &FreezeCapability<CoinType>
    ) {
        // TODO(fa_migration)
        pragma verify = false;
        // pragma opaque;
        modifies global<CoinStore<CoinType>>(account_addr);
        /// [high-level-req-6.4]
        aborts_if !exists<CoinStore<CoinType>>(account_addr);
        let post coin_store = global<CoinStore<CoinType>>(account_addr);
        ensures !coin_store.frozen;
    }

    /// The creator of `CoinType` must be `@aptos_framework`.
    /// `SupplyConfig` allow upgrade.
    spec upgrade_supply<CoinType>(account: &signer) {
        // aborts_if true;
        pragma verify = false;
    }

    spec initialize {
        pragma opaque = true;
        let account_addr = signer::address_of(account);
        /// [high-level-req-1.2]
        aborts_if type_info::type_of<CoinType>().account_address != account_addr;
        /// [high-level-req-2]
        aborts_if exists<CoinInfo<CoinType>>(account_addr);
        aborts_if string::length(name) > MAX_COIN_NAME_LENGTH;
        aborts_if string::length(symbol) > MAX_COIN_SYMBOL_LENGTH;
    }

    // `account` must be `@aptos_framework`.
    spec initialize_with_parallelizable_supply<CoinType>(
        account: &signer,
        name: string::String,
        symbol: string::String,
        decimals: u8,
        monitor_supply: bool
    ): (
        BurnCapability<CoinType>, FreezeCapability<CoinType>, MintCapability<CoinType>
    ) {
        use aptos_framework::aggregator_factory;
        let addr = signer::address_of(account);
        aborts_if addr != @aptos_framework;
        aborts_if monitor_supply
            && !exists<aggregator_factory::AggregatorFactory>(@aptos_framework);
        include InitializeInternalSchema<CoinType> { name: name.bytes, symbol: symbol.bytes };
        ensures exists<CoinInfo<CoinType>>(addr);
    }

    /// Make sure `name` and `symbol` are legal length.
    /// Only the creator of `CoinType` can initialize.
    spec schema InitializeInternalSchema<CoinType> {
        account: signer;
        name: vector<u8>;
        symbol: vector<u8>;
        let account_addr = signer::address_of(account);
        let coin_address = type_info::type_of<CoinType>().account_address;
        aborts_if coin_address != account_addr;
        aborts_if exists<CoinInfo<CoinType>>(account_addr);
        aborts_if len(name) > MAX_COIN_NAME_LENGTH;
        aborts_if len(symbol) > MAX_COIN_SYMBOL_LENGTH;
    }

    spec initialize_internal<CoinType>(
        account: &signer,
        name: string::String,
        symbol: string::String,
        decimals: u8,
        monitor_supply: bool,
        parallelizable: bool
    ): (
        BurnCapability<CoinType>, FreezeCapability<CoinType>, MintCapability<CoinType>
    ) {
        use aptos_framework::aggregator_factory;
        pragma opaque = true;
        include InitializeInternalSchema<CoinType> { name: name.bytes, symbol: symbol.bytes };
        let account_addr = signer::address_of(account);
        let post coin_info = global<CoinInfo<CoinType>>(account_addr);
        let post supply = option::borrow(coin_info.supply);
        let post value = optional_aggregator::optional_aggregator_value(supply);
        let post limit = optional_aggregator::optional_aggregator_limit(supply);
        modifies global<CoinInfo<CoinType>>(account_addr);
        aborts_if monitor_supply
            && parallelizable
            && !exists<aggregator_factory::AggregatorFactory>(@aptos_framework);
        /// [managed_coin::high-level-req-2]
        ensures exists<CoinInfo<CoinType>>(account_addr)
            && coin_info.name == name
            && coin_info.symbol == symbol && coin_info.decimals == decimals;
        ensures if (monitor_supply) {
            value == 0
                && limit == MAX_U128
                && (parallelizable == optional_aggregator::is_parallelizable(supply))
        } else {
            option::is_none(coin_info.supply)
        };
        ensures result_1 == BurnCapability<CoinType> {};
        ensures result_2 == FreezeCapability<CoinType> {};
        ensures result_3 == MintCapability<CoinType> {};
    }

    spec merge<CoinType>(
        dst_coin: &mut Coin<CoinType>, source_coin: Coin<CoinType>
    ) {
        pragma opaque;
        pragma aborts_if_is_partial = false;
        aborts_if dst_coin.value + source_coin.value > MAX_U64;
        /// [high-level-req-3]
        ensures dst_coin.value == old(dst_coin.value) + source_coin.value;
        ensures supply<CoinType> == old(supply<CoinType>);
    }

    /// An account can only be registered once.
    /// Updating `Account.guid_creation_num` will not overflow.
    spec register<CoinType>(account: &signer) {
        use 0x1::signer;
        pragma opaque = true, aborts_if_is_partial = false;
        // let account_addr = signer::address_of(account);
        // let acc = global<account::Account>(account_addr);
        // aborts_if !exists<CoinStore<CoinType>>(account_addr) && acc.guid_creation_num + 2 >= account::MAX_GUID_CREATION_NUM;
        // aborts_if !exists<CoinStore<CoinType>>(account_addr) && acc.guid_creation_num + 2 > MAX_U64;
        // aborts_if !exists<CoinStore<CoinType>>(account_addr) && !exists<account::Account>(account_addr);
        // aborts_if !exists<CoinStore<CoinType>>(account_addr) && !type_info::spec_is_struct<CoinType>();
        let account_addr = signer::address_of(account);
        aborts_if aborts_of<is_account_registered<CoinType>> (account_addr);
    }

    /// `from` and `to` account not frozen.
    /// `from` and `to` not the same address.
    /// `from` account sufficient balance.
    spec transfer<CoinType>(from: &signer, to: address, amount: u64) {
        // TODO(fa_migration)
        pragma verify = false;
        let account_addr_from = signer::address_of(from);
        let coin_store_from = global<CoinStore<CoinType>>(account_addr_from);
        let post coin_store_post_from = global<CoinStore<CoinType>>(account_addr_from);
        let coin_store_to = global<CoinStore<CoinType>>(to);
        let post coin_store_post_to = global<CoinStore<CoinType>>(to);

        /// [high-level-req-6.5]
        aborts_if !exists<CoinStore<CoinType>>(account_addr_from);
        aborts_if !exists<CoinStore<CoinType>>(to);
        /// [high-level-req-8.2]
        aborts_if coin_store_from.frozen;
        aborts_if coin_store_to.frozen;
        aborts_if coin_store_from.coin.value < amount;

        ensures account_addr_from != to ==>
            coin_store_post_from.coin.value == coin_store_from.coin.value - amount;
        ensures account_addr_from != to ==>
            coin_store_post_to.coin.value == coin_store_to.coin.value + amount;
        ensures account_addr_from == to ==>
            coin_store_post_from.coin.value == coin_store_from.coin.value;
    }

    /// Account is not frozen and sufficient balance.
    spec withdraw<CoinType>(account: &signer, amount: u64): Coin<CoinType> {
        pragma opaque = true;
        // TODO(fa_migration)
        pragma verify = false;
        include WithdrawAbortsIf<CoinType>;
        modifies global<CoinStore<CoinType>>(account_addr);
        let account_addr = signer::address_of(account);
        let coin_store = global<CoinStore<CoinType>>(account_addr);
        let balance = coin_store.coin.value;
        let post coin_post = global<CoinStore<CoinType>>(account_addr).coin.value;
        ensures coin_post == balance - amount;
        ensures result == Coin<CoinType> { value: amount };
    }

    spec schema WithdrawAbortsIf<CoinType> {
        account: &signer;
        amount: u64;
        let account_addr = signer::address_of(account);
        let coin_store = global<CoinStore<CoinType>>(account_addr);
        let balance = coin_store.coin.value;
        /// [high-level-req-6.6]
        aborts_if !exists<CoinStore<CoinType>>(account_addr);
        /// [high-level-req-8.1]
        aborts_if coin_store.frozen;
        aborts_if balance < amount;
    }

    spec destroy_freeze_cap {
        pragma opaque = true;
    }

    spec destroy_mint_cap {
        pragma opaque = true;
    }

    spec ensure_paired_metadata {
        use 0x1::fungible_asset;
        use 0x1::object;
        use 0x1::primary_fungible_store;
        pragma opaque = true, aborts_if_is_partial = false;
        let metadata_addr = spec_paired_metadata_address<CoinType>();
        modifies CoinConversionMap[@aptos_framework];
        modifies PairedCoinType[metadata_addr];
        modifies PairedFungibleAssetRefs[metadata_addr];
        modifies object::ObjectCore[metadata_addr];
        modifies fungible_asset::Metadata[metadata_addr];
        modifies fungible_asset::Supply[metadata_addr];
        modifies fungible_asset::ConcurrentSupply[metadata_addr];
        modifies primary_fungible_store::DeriveRefPod[metadata_addr];
        ensures result
            == result_of<create_and_return_paired_metadata_if_not_exist<CoinType>> (false);
        ensures object::object_address(result) == metadata_addr;
        aborts_if aborts_of<create_and_return_paired_metadata_if_not_exist<CoinType>> (false);
    }

    spec value<CoinType>(coin: &Coin<CoinType>): u64 {
        pragma opaque = true;
        pragma aborts_if_is_partial = false;
        ensures result == coin.value;
        aborts_if false;
    }

    spec zero<CoinType>(): Coin<CoinType> {
        pragma opaque = true;
        pragma aborts_if_is_partial = false;
        ensures result == Coin<CoinType> { value: 0 };
        aborts_if false;
    }
}
