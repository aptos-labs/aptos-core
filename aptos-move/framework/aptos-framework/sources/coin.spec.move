spec aptos_framework::coin {
    spec module {
        pragma verify = true;
        global supply<CoinType>: num;
        global aggregate_supply<CoinType>: num;
        apply TotalSupplyTracked<CoinType> to *<CoinType> except
            initialize, initialize_internal, initialize_with_parallelizable_supply;
        apply TotalSupplyNoChange<CoinType> to *<CoinType> except mint,
            burn, burn_from, initialize, initialize_internal, initialize_with_parallelizable_supply;
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
        use aptos_framework::aggregator;
        invariant aggregator::spec_get_limit(value) == MAX_U64;
    }

    spec mint {
        pragma opaque;
        let addr = type_info::type_of<CoinType>().account_address;
        modifies global<CoinInfo<CoinType>>(addr);
        aborts_if [abstract] false;
        ensures [abstract] result.value == amount;
    }

    /// Get address by reflection.
    spec coin_address<CoinType>(): address {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == type_info::type_of<CoinType>().account_address;
    }

    /// Can only be initialized once.
    /// Can only be published by reserved addresses.
    spec initialize_supply_config(aptos_framework: &signer) {
        let aptos_addr = signer::address_of(aptos_framework);
        aborts_if !system_addresses::is_aptos_framework_address(aptos_addr);
        aborts_if exists<SupplyConfig>(aptos_addr);
        ensures !global<SupplyConfig>(aptos_addr).allow_upgrades;
        ensures exists<SupplyConfig>(aptos_addr);
    }

    /// Can only be updated by `@aptos_framework`.
    spec allow_supply_upgrades(aptos_framework: &signer, allowed: bool) {
        modifies global<SupplyConfig>(@aptos_framework);
        let aptos_addr = signer::address_of(aptos_framework);
        aborts_if !system_addresses::is_aptos_framework_address(aptos_addr);
        aborts_if !exists<SupplyConfig>(aptos_addr);
        let post allow_upgrades_post = global<SupplyConfig>(@aptos_framework);
        ensures allow_upgrades_post.allow_upgrades == allowed;
    }

    spec balance<CoinType>(owner: address): u64 {
        aborts_if !exists<CoinStore<CoinType>>(owner);
        ensures result == global<CoinStore<CoinType>>(owner).coin.value;
    }

    spec is_coin_initialized<CoinType>(): bool {
        pragma verify = false;
    }

    spec fun get_coin_supply_opt<CoinType>(): Option<OptionalAggregator> {
        global<CoinInfo<CoinType>>(type_info::type_of<CoinType>().account_address).supply
    }

    spec schema AbortsIfAggregator<CoinType> {
        use aptos_framework::optional_aggregator;
        use aptos_framework::aggregator;
        coin: Coin<CoinType>;
        let addr =  type_info::type_of<CoinType>().account_address;
        let maybe_supply = global<CoinInfo<CoinType>>(addr).supply;
        aborts_if option::is_some(maybe_supply) && optional_aggregator::is_parallelizable(option::borrow(maybe_supply))
            && aggregator::spec_aggregator_get_val(option::borrow(option::borrow(maybe_supply).aggregator)) <
            coin.value;
        aborts_if option::is_some(maybe_supply) && !optional_aggregator::is_parallelizable(option::borrow(maybe_supply))
            && option::borrow(option::borrow(maybe_supply).integer).value <
            coin.value;
    }

    spec schema AbortsIfNotExistCoinInfo<CoinType> {
        let addr = type_info::type_of<CoinType>().account_address;
        aborts_if !exists<CoinInfo<CoinType>>(addr);
    }

    spec name<CoinType>(): string::String {
        include AbortsIfNotExistCoinInfo<CoinType>;
    }

    spec symbol<CoinType>(): string::String {
        include AbortsIfNotExistCoinInfo<CoinType>;
    }

    spec decimals<CoinType>(): u8 {
        include AbortsIfNotExistCoinInfo<CoinType>;
    }

    spec supply<CoinType>(): Option<u128> {
        let coin_addr = type_info::type_of<CoinType>().account_address;
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
        let addr =  type_info::type_of<CoinType>().account_address;
        aborts_if !exists<CoinInfo<CoinType>>(addr);
        modifies global<CoinInfo<CoinType>>(addr);
        include AbortsIfNotExistCoinInfo<CoinType>;
        aborts_if coin.value == 0;
        include AbortsIfAggregator<CoinType>;
    }

    spec burn_from<CoinType>(
        account_addr: address,
        amount: u64,
        burn_cap: &BurnCapability<CoinType>,
    ) {
        let addr = type_info::type_of<CoinType>().account_address;
        let coin_store = global<CoinStore<CoinType>>(account_addr);
        let post post_coin_store = global<CoinStore<CoinType>>(account_addr);

        modifies global<CoinInfo<CoinType>>(addr);
        modifies global<CoinStore<CoinType>>(account_addr);

        aborts_if amount != 0 && !exists<CoinInfo<CoinType>>(addr);
        aborts_if amount != 0 && !exists<CoinStore<CoinType>>(account_addr);
        aborts_if coin_store.coin.value < amount;

        let maybe_supply = global<CoinInfo<CoinType>>(addr).supply;
        let supply = option::spec_borrow(maybe_supply);
        let value = optional_aggregator::optional_aggregator_value(supply);

        let post post_maybe_supply = global<CoinInfo<CoinType>>(addr).supply;
        let post post_supply = option::spec_borrow(post_maybe_supply);
        let post post_value = optional_aggregator::optional_aggregator_value(post_supply);

        aborts_if option::spec_is_some(maybe_supply) && value < amount;

        ensures post_coin_store.coin.value == coin_store.coin.value - amount;
        ensures if (option::spec_is_some(maybe_supply)) {
            post_value == value - amount
        } else {
            option::spec_is_none(post_maybe_supply)
        };
    }

    /// `account_addr` is not frozen.
    spec deposit<CoinType>(account_addr: address, coin: Coin<CoinType>) {
        modifies global<CoinInfo<CoinType>>(account_addr);
        ensures global<CoinStore<CoinType>>(account_addr).coin.value == old(global<CoinStore<CoinType>>(account_addr)).coin.value + coin.value;
    }
    spec schema DepositAbortsIf<CoinType> {
        account_addr: address;
        coin: Coin<CoinType>;
        let coin_store = global<CoinStore<CoinType>>(account_addr);
        aborts_if !exists<CoinStore<CoinType>>(account_addr);
        aborts_if coin_store.frozen;
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
        pragma opaque;
        modifies global<CoinStore<CoinType>>(account_addr);
        aborts_if !exists<CoinStore<CoinType>>(account_addr);
        let post coin_store = global<CoinStore<CoinType>>(account_addr);
        ensures coin_store.frozen;
    }

    spec unfreeze_coin_store<CoinType>(
        account_addr: address,
        _freeze_cap: &FreezeCapability<CoinType>,
    ) {
        pragma opaque;
        modifies global<CoinStore<CoinType>>(account_addr);
        aborts_if !exists<CoinStore<CoinType>>(account_addr);
        let post coin_store = global<CoinStore<CoinType>>(account_addr);
        ensures !coin_store.frozen;
    }

    /// The creator of `CoinType` must be `@aptos_framework`.
    /// `SupplyConfig` allow upgrade.
    spec upgrade_supply<CoinType>(account: &signer) {
        let account_addr = signer::address_of(account);
        let coin_address = type_info::type_of<CoinType>().account_address;
        aborts_if coin_address != account_addr;
        aborts_if !exists<SupplyConfig>(@aptos_framework);
        aborts_if !exists<CoinInfo<CoinType>>(account_addr);

        let supply_config = global<SupplyConfig>(@aptos_framework);
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

        aborts_if supply_no_parallel && !exists<aggregator_factory::AggregatorFactory>(@aptos_framework);
        ensures supply_no_parallel ==>
            optional_aggregator::is_parallelizable(post_supply) && post_value == value;
    }

    spec initialize {
        let account_addr = signer::address_of(account);
        aborts_if type_info::type_of<CoinType>().account_address != account_addr;
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
        monitor_supply: bool,
    ): (BurnCapability<CoinType>, FreezeCapability<CoinType>, MintCapability<CoinType>) {
        pragma aborts_if_is_partial;
        let addr = signer::address_of(account);
        aborts_if addr != @aptos_framework;
        include InitializeInternalSchema<CoinType>{
            name: name.bytes,
            symbol: symbol.bytes
        };
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
        include InitializeInternalSchema<CoinType>{
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
            && !exists<aggregator_factory::AggregatorFactory>(@aptos_framework);
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
        ensures dst_coin.value == old(dst_coin.value) + source_coin.value;
    }

    /// An account can only be registered once.
    /// Updating `Account.guid_creation_num` will not overflow.
    spec register<CoinType>(account: &signer) {
        let account_addr = signer::address_of(account);
        let acc = global<account::Account>(account_addr);
        aborts_if !exists<CoinStore<CoinType>>(account_addr) && acc.guid_creation_num + 2 >= account::MAX_GUID_CREATION_NUM;
        aborts_if !exists<CoinStore<CoinType>>(account_addr) && acc.guid_creation_num + 2 > MAX_U64;
        aborts_if !exists<CoinStore<CoinType>>(account_addr) && !exists<account::Account>(account_addr);
        aborts_if !exists<CoinStore<CoinType>>(account_addr) && !type_info::spec_is_struct<CoinType>();
        ensures exists<CoinStore<CoinType>>(account_addr);
    }

    /// `from` and `to` account not frozen.
    /// `from` and `to` not the same address.
    /// `from` account sufficient balance.
    spec transfer<CoinType>(
        from: &signer,
        to: address,
        amount: u64,
    ) {
        let account_addr_from = signer::address_of(from);
        let coin_store_from = global<CoinStore<CoinType>>(account_addr_from);
        let post coin_store_post_from = global<CoinStore<CoinType>>(account_addr_from);
        let coin_store_to = global<CoinStore<CoinType>>(to);
        let post coin_store_post_to = global<CoinStore<CoinType>>(to);

        aborts_if !exists<CoinStore<CoinType>>(account_addr_from);
        aborts_if !exists<CoinStore<CoinType>>(to);
        aborts_if coin_store_from.frozen;
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
        include WithdrawAbortsIf<CoinType>;
        modifies global<CoinStore<CoinType>>(account_addr);
        let account_addr = signer::address_of(account);
        let coin_store = global<CoinStore<CoinType>>(account_addr);
        let balance = coin_store.coin.value;
        let post coin_post = global<CoinStore<CoinType>>(account_addr).coin.value;
        ensures coin_post == balance - amount;
        ensures result == Coin<CoinType>{value: amount};
    }
    spec schema WithdrawAbortsIf<CoinType> {
        account: &signer;
        amount: u64;
        let account_addr = signer::address_of(account);
        let coin_store = global<CoinStore<CoinType>>(account_addr);
        let balance = coin_store.coin.value;
        aborts_if !exists<CoinStore<CoinType>>(account_addr);
        aborts_if coin_store.frozen;
        aborts_if balance < amount;
    }

    spec initialize_aggregatable_coin<CoinType>(aptos_framework: &signer): AggregatableCoin<CoinType> {
        include system_addresses::AbortsIfNotAptosFramework{account: aptos_framework};
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
        aborts_if aggregator::spec_aggregator_get_val(aggr)
            + coin.value > aggregator::spec_get_limit(aggr);
        aborts_if aggregator::spec_aggregator_get_val(aggr)
            + coin.value > MAX_U128;
    }

    spec collect_into_aggregatable_coin<CoinType>(account_addr: address, amount: u64, dst_coin: &mut AggregatableCoin<CoinType>) {
        let aggr = dst_coin.value;
        let coin_store = global<CoinStore<CoinType>>(account_addr);
        aborts_if amount > 0 && !exists<CoinStore<CoinType>>(account_addr);
        aborts_if amount > 0 && coin_store.coin.value < amount;
        aborts_if amount > 0 && aggregator::spec_aggregator_get_val(aggr)
            + amount > aggregator::spec_get_limit(aggr);
        aborts_if amount > 0 && aggregator::spec_aggregator_get_val(aggr)
            + amount > MAX_U128;
    }
}
