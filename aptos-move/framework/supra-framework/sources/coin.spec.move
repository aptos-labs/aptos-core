spec supra_framework::coin {
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
        global supply<CoinType>: num;
        global aggregate_supply<CoinType>: num;
        apply TotalSupplyTracked<CoinType> to *<CoinType> except
        initialize, initialize_internal, initialize_with_parallelizable_supply, initialize_internal_with_limit, initialize_with_parallelizable_supply_with_limit ;
        // TODO(fa_migration)
        // apply TotalSupplyNoChange<CoinType> to *<CoinType> except mint,
        //     burn, burn_from, initialize, initialize_internal, initialize_with_parallelizable_supply;
    }

    spec fun spec_fun_supply_tracked<CoinType>(val: u64, supply: Option<OptionalAggregator>): bool {
        option::spec_is_some(supply) ==> val == optional_aggregator::optional_aggregator_value
            (option::spec_borrow(supply))
    }

    spec schema TotalSupplyTracked<CoinType> {
        ensures old(spec_fun_supply_tracked<CoinType>(supply<CoinType> + aggregate_supply<CoinType>,
            global<CoinInfo<CoinType>>(type_info::type_of<CoinType>().account_address).supply)) ==>
            spec_fun_supply_tracked<CoinType>(supply<CoinType> + aggregate_supply<CoinType>,
                global<CoinInfo<CoinType>>(type_info::type_of<CoinType>().account_address).supply);
    }

    spec fun spec_fun_supply_no_change<CoinType>(old_supply: Option<OptionalAggregator>,
                                                 supply: Option<OptionalAggregator>): bool {
        option::spec_is_some(old_supply) ==> optional_aggregator::optional_aggregator_value
            (option::spec_borrow(old_supply)) == optional_aggregator::optional_aggregator_value
            (option::spec_borrow(supply))
    }

    spec schema TotalSupplyNoChange<CoinType> {
        let old_supply = global<CoinInfo<CoinType>>(type_info::type_of<CoinType>().account_address).supply;
        let post supply = global<CoinInfo<CoinType>>(type_info::type_of<CoinType>().account_address).supply;
        ensures spec_fun_supply_no_change<CoinType>(old_supply, supply);
    }

    spec AggregatableCoin {
        use supra_framework::aggregator;
        invariant aggregator::spec_get_limit(value) == MAX_U64;
    }

    spec mint {
        let addr = type_info::type_of<CoinType>().account_address;
        modifies global<CoinInfo<CoinType>>(addr);
    }

    spec mint_internal {
        let addr = type_info::type_of<CoinType>().account_address;
        modifies global<CoinInfo<CoinType>>(addr);
        aborts_if (amount != 0) && !exists<CoinInfo<CoinType>>(addr);
        ensures supply<CoinType> == old(supply<CoinType>) + amount;
        ensures result.value == amount;
    }

    /// Get address by reflection.
    spec coin_address<CoinType>(): address {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == type_info::type_of<CoinType>().account_address;
    }

    /// Can only be initialized once.
    /// Can only be published by reserved addresses.
    spec initialize_supply_config(supra_framework: &signer) {
        let aptos_addr = signer::address_of(supra_framework);
        aborts_if !system_addresses::is_supra_framework_address(aptos_addr);
        aborts_if exists<SupplyConfig>(aptos_addr);
        ensures !global<SupplyConfig>(aptos_addr).allow_upgrades;
        ensures exists<SupplyConfig>(aptos_addr);
    }

    /// Can only be updated by `@supra_framework`.
    spec allow_supply_upgrades(supra_framework: &signer, allowed: bool) {
        modifies global<SupplyConfig>(@supra_framework);
        let aptos_addr = signer::address_of(supra_framework);
        aborts_if !system_addresses::is_supra_framework_address(aptos_addr);
        aborts_if !exists<SupplyConfig>(aptos_addr);
        let post allow_upgrades_post = global<SupplyConfig>(@supra_framework);
        ensures allow_upgrades_post.allow_upgrades == allowed;
    }

    spec balance<CoinType>(owner: address): u64 {
        // TODO(fa_migration)
        pragma verify = false;
        aborts_if !exists<CoinStore<CoinType>>(owner);
        ensures result == global<CoinStore<CoinType>>(owner).coin.value;
    }

    spec is_coin_initialized<CoinType>(): bool {
        /// [high-level-req-7.1]
        aborts_if false;
    }

    spec is_account_registered<CoinType>(account_addr: address): bool {
        pragma aborts_if_is_partial;
        aborts_if false;
    }

    spec fun get_coin_supply_opt<CoinType>(): Option<OptionalAggregator> {
        global<CoinInfo<CoinType>>(type_info::type_of<CoinType>().account_address).supply
    }

    spec fun spec_paired_metadata<CoinType>(): Option<Object<Metadata>> {
        if (exists<CoinConversionMap>(@supra_framework)) {
            let map = global<CoinConversionMap>(@supra_framework).coin_to_fungible_asset_map;
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

    spec fun spec_is_account_registered<CoinType>(account_addr: address): bool {
        let paired_metadata_opt = spec_paired_metadata<CoinType>();
        exists<CoinStore<CoinType>>(account_addr) || (option::spec_is_some(
            paired_metadata_opt
        ) && primary_fungible_store::spec_primary_store_exists(account_addr, option::spec_borrow(paired_metadata_opt)))
    }

    spec schema CoinSubAbortsIf<CoinType> {
        use supra_framework::optional_aggregator;
        amount: u64;
        let addr = type_info::type_of<CoinType>().account_address;
        let maybe_supply = global<CoinInfo<CoinType>>(addr).supply;
        include (option::is_some(
            maybe_supply
        )) ==> optional_aggregator::SubAbortsIf { optional_aggregator: option::borrow(maybe_supply), value: amount };
    }

    spec schema CoinAddAbortsIf<CoinType> {
        use supra_framework::optional_aggregator;
        amount: u64;
        let addr = type_info::type_of<CoinType>().account_address;
        let maybe_supply = global<CoinInfo<CoinType>>(addr).supply;
        include (option::is_some(
            maybe_supply
        )) ==> optional_aggregator::AddAbortsIf { optional_aggregator: option::borrow(maybe_supply), value: amount };
    }

    spec schema AbortsIfNotExistCoinInfo<CoinType> {
        let addr = type_info::type_of<CoinType>().account_address;
        aborts_if !exists<CoinInfo<CoinType>>(addr);
    }

    spec name<CoinType>(): string::String {
        /// [high-level-req-7.3]
        include AbortsIfNotExistCoinInfo<CoinType>;
    }

    spec symbol<CoinType>(): string::String {
        /// [high-level-req-7.4]
        include AbortsIfNotExistCoinInfo<CoinType>;
    }

    spec decimals<CoinType>(): u8 {
        include AbortsIfNotExistCoinInfo<CoinType>;
    }

    spec supply<CoinType>(): Option<u128> {
        // TODO(fa_migration)
        pragma verify = true;
    }

    spec coin_supply<CoinType>(): Option<u128> {
        let coin_addr = type_info::type_of<CoinType>().account_address;
        /// [high-level-req-7.5]
        aborts_if !exists<CoinInfo<CoinType>>(coin_addr);
        let maybe_supply = global<CoinInfo<CoinType>>(coin_addr).supply;
        let supply = option::spec_borrow(maybe_supply);
        let value = optional_aggregator::optional_aggregator_value(supply);

        ensures if (option::spec_is_some(maybe_supply)) {
            result == option::spec_some(value)
        } else {
            option::spec_is_none(result)
        };
    }

    spec burn<CoinType>(
    coin: Coin<CoinType>,
    _cap: &BurnCapability<CoinType>,
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
        // TODO(fa_migration)
        pragma verify = true;
        let addr = type_info::type_of<CoinType>().account_address;
        modifies global<CoinInfo<CoinType>>(addr);
    }

    spec burn_from<CoinType>(
    account_addr: address,
    amount: u64,
    burn_cap: &BurnCapability<CoinType>,
    ) {
        // TODO(fa_migration)
        pragma verify = true;
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
        let supply_aggr = option::spec_borrow(maybe_supply);
        let value = optional_aggregator::optional_aggregator_value(supply_aggr);

        let post post_maybe_supply = global<CoinInfo<CoinType>>(addr).supply;
        let post post_supply = option::spec_borrow(post_maybe_supply);
        let post post_value = optional_aggregator::optional_aggregator_value(post_supply);

        aborts_if option::spec_is_some(maybe_supply) && value < amount;

        ensures post_coin_store.coin.value == coin_store.coin.value - amount;
        /// [managed_coin::high-level-req-5]
        ensures if (option::spec_is_some(maybe_supply)) {
            post_value == value - amount
        } else {
            option::spec_is_none(post_maybe_supply)
        };
        ensures supply<CoinType> == old(supply<CoinType>) - amount;
    }

    /// `account_addr` is not frozen.
    spec deposit<CoinType>(account_addr: address, coin: Coin<CoinType>) {
        // TODO(fa_migration)
        pragma verify = true;
        // can not make this opaque because fa
        // pragma opaque;
        // modifies global<CoinInfo<CoinType>>(account_addr);
        /// [high-level-req-8.3]
        include DepositAbortsIf<CoinType>;
        ensures global<CoinStore<CoinType>>(account_addr).coin.value == old(
            global<CoinStore<CoinType>>(account_addr)
        ).coin.value + coin.value;
    }

    spec coin_to_fungible_asset<CoinType>(coin: Coin<CoinType>): FungibleAsset {
        // TODO(fa_migration)
        pragma verify = false;
        let addr = type_info::type_of<CoinType>().account_address;
        // Comment out frame because it is not verified at all.
        // modifies global<CoinInfo<CoinType>>(addr);
    }

    spec coin_to_fungible_asset_internal {
        // TODO(fa_migration)
        pragma verify = false;
        // modifies global<CoinInfo<CoinType>>(account);
    }

    spec fungible_asset_to_coin<CoinType>(fungible_asset: FungibleAsset): Coin<CoinType> {
        // TODO(fa_migration)
        pragma verify = false;
    }

    spec maybe_convert_to_fungible_store<CoinType>(account: address) {
        // TODO(fa_migration)
        pragma verify = false;
        // Comment out frame because it is not verified at all.
        // modifies global<CoinStore<CoinType>>(account);
    }

    spec schema DepositAbortsIf<CoinType> {
        account_addr: address;
        let coin_store = global<CoinStore<CoinType>>(account_addr);
        aborts_if !exists<CoinStore<CoinType>>(account_addr);
        aborts_if coin_store.frozen;
    }

    spec force_deposit<CoinType>(account_addr: address, coin: Coin<CoinType>) {
        // TODO(fa_migration)
        pragma verify = false;
        modifies global<CoinStore<CoinType>>(account_addr);
        aborts_if !exists<CoinStore<CoinType>>(account_addr);
        ensures global<CoinStore<CoinType>>(account_addr).coin.value == old(
            global<CoinStore<CoinType>>(account_addr)
        ).coin.value + coin.value;
    }

    /// The value of `zero_coin` must be 0.
    spec destroy_zero<CoinType>(zero_coin: Coin<CoinType>) {
        aborts_if zero_coin.value > 0;
    }

    spec extract<CoinType>(coin: &mut Coin<CoinType>, amount: u64): Coin<CoinType> {
        aborts_if coin.value < amount;
        ensures result.value == amount;
        ensures coin.value == old(coin.value) - amount;
    }

    spec extract_all<CoinType>(coin: &mut Coin<CoinType>): Coin<CoinType> {
        ensures result.value == old(coin).value;
        ensures coin.value == 0;
    }

    spec freeze_coin_store<CoinType>(
    account_addr: address,
    _freeze_cap: &FreezeCapability<CoinType>,
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
    account_addr: address,
    _freeze_cap: &FreezeCapability<CoinType>,
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

    /// The creator of `CoinType` must be `@supra_framework`.
    /// `SupplyConfig` allow upgrade.
    spec upgrade_supply<CoinType>(account: &signer) {
        let account_addr = signer::address_of(account);
        let coin_address = type_info::type_of<CoinType>().account_address;
        aborts_if coin_address != account_addr;
        aborts_if !exists<SupplyConfig>(@supra_framework);
        /// [high-level-req-1.1]
        aborts_if !exists<CoinInfo<CoinType>>(account_addr);

        let supply_config = global<SupplyConfig>(@supra_framework);
        aborts_if !supply_config.allow_upgrades;
        modifies global<CoinInfo<CoinType>>(account_addr);

        let maybe_supply = global<CoinInfo<CoinType>>(account_addr).supply;
        let supply = option::spec_borrow(maybe_supply);
        let value = optional_aggregator::optional_aggregator_value(supply);

        let post post_maybe_supply = global<CoinInfo<CoinType>>(account_addr).supply;
        let post post_supply = option::spec_borrow(post_maybe_supply);
        let post post_value = optional_aggregator::optional_aggregator_value(post_supply);

        let supply_no_parallel = option::spec_is_some(maybe_supply) &&
            !optional_aggregator::is_parallelizable(supply);

        aborts_if supply_no_parallel && !exists<aggregator_factory::AggregatorFactory>(@supra_framework);
        ensures supply_no_parallel ==>
            optional_aggregator::is_parallelizable(post_supply) && post_value == value;
    }

    spec initialize {
        let account_addr = signer::address_of(account);
        /// [high-level-req-1.2]
        aborts_if type_info::type_of<CoinType>().account_address != account_addr;
        /// [high-level-req-2]
        aborts_if exists<CoinInfo<CoinType>>(account_addr);
        aborts_if string::length(name) > MAX_COIN_NAME_LENGTH;
        aborts_if string::length(symbol) > MAX_COIN_SYMBOL_LENGTH;
    }

    // `account` must be `@supra_framework`.
    spec initialize_with_parallelizable_supply<CoinType>(
    account: &signer,
    name: string::String,
    symbol: string::String,
    decimals: u8,
    monitor_supply: bool,
    ): (BurnCapability<CoinType>, FreezeCapability<CoinType>, MintCapability<CoinType>) {
        use supra_framework::aggregator_factory;
        let addr = signer::address_of(account);
        aborts_if addr != @supra_framework;
        aborts_if monitor_supply && !exists<aggregator_factory::AggregatorFactory>(@supra_framework);
        include InitializeInternalSchema<CoinType> {
            name: name.bytes,
            symbol: symbol.bytes
        };
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
    parallelizable: bool,
    ): (BurnCapability<CoinType>, FreezeCapability<CoinType>, MintCapability<CoinType>) {
        include InitializeInternalSchema<CoinType> {
            name: name.bytes,
            symbol: symbol.bytes
        };
        let account_addr = signer::address_of(account);
        let post coin_info = global<CoinInfo<CoinType>>(account_addr);
        let post supply = option::spec_borrow(coin_info.supply);
        let post value = optional_aggregator::optional_aggregator_value(supply);
        let post limit = optional_aggregator::optional_aggregator_limit(supply);
        modifies global<CoinInfo<CoinType>>(account_addr);
        aborts_if monitor_supply && parallelizable
            && !exists<aggregator_factory::AggregatorFactory>(@supra_framework);
        /// [managed_coin::high-level-req-2]
        ensures exists<CoinInfo<CoinType>>(account_addr)
            && coin_info.name == name
            && coin_info.symbol == symbol
            && coin_info.decimals == decimals;
        ensures if (monitor_supply) {
            value == 0 && limit == MAX_U128
                && (parallelizable == optional_aggregator::is_parallelizable(supply))
        } else {
            option::spec_is_none(coin_info.supply)
        };
        ensures result_1 == BurnCapability<CoinType> {};
        ensures result_2 == FreezeCapability<CoinType> {};
        ensures result_3 == MintCapability<CoinType> {};
    }

    spec merge<CoinType>(dst_coin: &mut Coin<CoinType>, source_coin: Coin<CoinType>) {
        /// [high-level-req-3]
        ensures dst_coin.value == old(dst_coin.value) + source_coin.value;
    }

    /// An account can only be registered once.
    /// Updating `Account.guid_creation_num` will not overflow.
    spec register<CoinType>(account: &signer) {
        // TODO(fa_migration)
        pragma verify = false;
        // let account_addr = signer::address_of(account);
        // let acc = global<account::Account>(account_addr);
        // aborts_if !exists<CoinStore<CoinType>>(account_addr) && acc.guid_creation_num + 2 >= account::MAX_GUID_CREATION_NUM;
        // aborts_if !exists<CoinStore<CoinType>>(account_addr) && acc.guid_creation_num + 2 > MAX_U64;
        // aborts_if !exists<CoinStore<CoinType>>(account_addr) && !exists<account::Account>(account_addr);
        // aborts_if !exists<CoinStore<CoinType>>(account_addr) && !type_info::spec_is_struct<CoinType>();
        // ensures exists<CoinStore<CoinType>>(account_addr);
    }

    /// `from` and `to` account not frozen.
    /// `from` and `to` not the same address.
    /// `from` account sufficient balance.
    spec transfer<CoinType>(
    from: &signer,
    to: address,
    amount: u64,
    ) {
        // TODO(fa_migration)
        pragma verify = true;
        pragma aborts_if_is_partial;
        let account_addr_from = signer::address_of(from);
        let coin_store_from = global<CoinStore<CoinType>>(account_addr_from);
        let post coin_store_post_from = global<CoinStore<CoinType>>(account_addr_from);
        let coin_store_to = global<CoinStore<CoinType>>(to);
        let post coin_store_post_to = global<CoinStore<CoinType>>(to);

        // The two comment out aborts conditions are related to withdraw, which subject fa migration.
        /// [high-level-req-6.5]
        // aborts_if !exists<CoinStore<CoinType>>(account_addr_from);
        aborts_if !exists<CoinStore<CoinType>>(to);
        /// [high-level-req-8.2]
        // aborts_if coin_store_from.frozen;
        aborts_if coin_store_to.frozen;
        aborts_if coin_store_from.coin.value < amount;

        ensures account_addr_from != to ==> coin_store_post_from.coin.value ==
            coin_store_from.coin.value - amount;
        ensures account_addr_from != to ==> coin_store_post_to.coin.value == coin_store_to.coin.value + amount;
        ensures account_addr_from == to ==> coin_store_post_from.coin.value == coin_store_from.coin.value;
    }

    /// Account is not frozen and sufficient balance.
    spec withdraw<CoinType>(
    account: &signer,
    amount: u64,
    ): Coin<CoinType> {
        // TODO(fa_migration)
        pragma verify = true;
        // include WithdrawAbortsIf<CoinType>;
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

    spec initialize_aggregatable_coin<CoinType>(supra_framework: &signer): AggregatableCoin<CoinType> {
        include system_addresses::AbortsIfNotSupraFramework { account: supra_framework };
        include aggregator_factory::CreateAggregatorInternalAbortsIf;
    }

    spec is_aggregatable_coin_zero<CoinType>(coin: &AggregatableCoin<CoinType>): bool {
        aborts_if false;
        ensures result == (aggregator::spec_read(coin.value) == 0);
    }

    spec drain_aggregatable_coin<CoinType>(coin: &mut AggregatableCoin<CoinType>): Coin<CoinType> {
        aborts_if aggregator::spec_read(coin.value) > MAX_U64;
        ensures result.value == aggregator::spec_aggregator_get_val(old(coin).value);
    }

    spec merge_aggregatable_coin<CoinType>(dst_coin: &mut AggregatableCoin<CoinType>, coin: Coin<CoinType>) {
        let aggr = dst_coin.value;
        let post p_aggr = dst_coin.value;
        aborts_if aggregator::spec_aggregator_get_val(aggr)
            + coin.value > aggregator::spec_get_limit(aggr);
        aborts_if aggregator::spec_aggregator_get_val(aggr)
            + coin.value > MAX_U128;
        ensures aggregator::spec_aggregator_get_val(aggr) + coin.value == aggregator::spec_aggregator_get_val(p_aggr);
    }

    spec collect_into_aggregatable_coin<CoinType>(account_addr: address, amount: u64, dst_coin: &mut AggregatableCoin<CoinType>) {
        // TODO(fa_migration)
        pragma verify = false;
        let aggr = dst_coin.value;
        let post p_aggr = dst_coin.value;
        let coin_store = global<CoinStore<CoinType>>(account_addr);
        let post p_coin_store = global<CoinStore<CoinType>>(account_addr);
        aborts_if amount > 0 && !exists<CoinStore<CoinType>>(account_addr);
        aborts_if amount > 0 && coin_store.coin.value < amount;
        aborts_if amount > 0 && aggregator::spec_aggregator_get_val(aggr)
            + amount > aggregator::spec_get_limit(aggr);
        aborts_if amount > 0 && aggregator::spec_aggregator_get_val(aggr)
            + amount > MAX_U128;
        ensures aggregator::spec_aggregator_get_val(aggr) + amount == aggregator::spec_aggregator_get_val(p_aggr);
        ensures coin_store.coin.value - amount == p_coin_store.coin.value;
    }
}
