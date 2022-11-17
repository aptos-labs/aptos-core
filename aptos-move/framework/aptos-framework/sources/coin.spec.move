spec aptos_framework::coin {
    spec module {
        pragma verify = true;
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

    spec schema ExistCoinInfo<CoinType> {
        let addr = type_info::type_of<CoinType>().account_address;
        aborts_if !exists<CoinInfo<CoinType>>(addr);
    }

    spec name<CoinType>(): string::String {
        include ExistCoinInfo<CoinType>;
    }

    spec symbol<CoinType>(): string::String {
        include ExistCoinInfo<CoinType>;
    }

    spec decimals<CoinType>(): u8 {
        include ExistCoinInfo<CoinType>;
    }

    spec supply<CoinType>(): Option<u128> {
        // TODO: complex aborts conditions.
        pragma aborts_if_is_partial;
        include ExistCoinInfo<CoinType>;
    }

    spec burn<CoinType>(
        coin: Coin<CoinType>,
        _cap: &BurnCapability<CoinType>,
    ) {
        // TODO: complex aborts conditions.
        pragma aborts_if_is_partial;
        let addr =  type_info::type_of<CoinType>().account_address;
        modifies global<CoinInfo<CoinType>>(addr);
        include ExistCoinInfo<CoinType>;
        aborts_if coin.value == 0;
    }

    spec burn_from<CoinType>(
        account_addr: address,
        amount: u64,
        burn_cap: &BurnCapability<CoinType>,
    ) {
        // TODO: complex aborts conditions.
        pragma aborts_if_is_partial;
        let addr =  type_info::type_of<CoinType>().account_address;
        modifies global<CoinInfo<CoinType>>(addr);
        aborts_if amount != 0 && !exists<CoinStore<CoinType>>(account_addr);
    }

    /// `account_addr` is not frozen.
    spec deposit<CoinType>(account_addr: address, coin: Coin<CoinType>) {
        let coin_store = global<CoinStore<CoinType>>(account_addr);
        aborts_if !exists<CoinStore<CoinType>>(account_addr);
        aborts_if coin_store.frozen;
        modifies global<CoinInfo<CoinType>>(account_addr);
        ensures global<CoinStore<CoinType>>(account_addr).coin.value == old(global<CoinStore<CoinType>>(account_addr)).coin.value + coin.value;
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
        // TODO: complex aborts conditions.
        pragma aborts_if_is_partial;
        let account_addr = signer::address_of(account);
        let coin_address = type_info::type_of<CoinType>().account_address;
        aborts_if coin_address != account_addr;
        aborts_if !exists<SupplyConfig>(@aptos_framework);
        aborts_if !exists<CoinInfo<CoinType>>(account_addr);
        let supply_config = global<SupplyConfig>(@aptos_framework);
        aborts_if !supply_config.allow_upgrades;
        modifies global<CoinInfo<CoinType>>(account_addr);
    }

    spec initialize {
        pragma verify = false;
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
        // TODO: complex aborts conditions
        pragma aborts_if_is_partial;
        include InitializeInternalSchema<CoinType>{
            name: name.bytes,
            symbol: symbol.bytes
        };
    }

    spec merge<CoinType>(dst_coin: &mut Coin<CoinType>, source_coin: Coin<CoinType>) {
        ensures dst_coin.value == old(dst_coin.value) + source_coin.value;
    }

    /// An account can only be registered once.
    /// Updating `Account.guid_creation_num` will not overflow.
    spec register<CoinType>(account: &signer) {
        // TODO: Add the abort condition about `type_info::type_of`
        pragma aborts_if_is_partial;
        let account_addr = signer::address_of(account);
        let acc = global<account::Account>(account_addr);
        aborts_if acc.guid_creation_num + 2 > MAX_U64;
        aborts_if exists<CoinStore<CoinType>>(account_addr);
        aborts_if !exists<account::Account>(account_addr);
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
        let account_addr = signer::address_of(account);
        let coin_store = global<CoinStore<CoinType>>(account_addr);
        let balance = coin_store.coin.value;
        aborts_if !exists<CoinStore<CoinType>>(account_addr);
        aborts_if coin_store.frozen;
        aborts_if balance < amount;

        modifies global<CoinStore<CoinType>>(account_addr);
        let post coin_post = global<CoinStore<CoinType>>(account_addr).coin.value;
        ensures coin_post == balance - amount;
        ensures result == Coin<CoinType>{value: amount};
    }
}
