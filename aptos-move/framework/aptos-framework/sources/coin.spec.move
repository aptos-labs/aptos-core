spec aptos_framework::coin {

    spec module {
        pragma verify = true;
    }

    spec mint {
        pragma opaque;
        let addr = spec_coin_address<CoinType>();
        modifies global<CoinInfo<CoinType>>(addr);
        aborts_if [abstract] false;
        ensures [abstract] result.value == amount;
    }

    spec coin_address {
        pragma opaque;
        pragma aborts_if_is_partial;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_coin_address<CoinType>();
    }

    spec fun spec_coin_address<CoinType>(): address {
        // TODO: abstracted due to the lack of support for `type_info` in Prover.
        @0x0
    }

    spec initialize_supply_config {
        let aptos_addr = signer::address_of(aptos_framework);
        aborts_if !system_addresses::is_aptos_framework_address(aptos_addr);
        aborts_if exists<SupplyConfig>(aptos_addr);
        ensures !global<SupplyConfig>(aptos_addr).allow_upgrades;
        ensures exists<SupplyConfig>(aptos_addr);
    }

    spec allow_supply_upgrades {
        let aptos_addr = signer::address_of(aptos_framework);
        aborts_if !system_addresses::is_aptos_framework_address(aptos_addr);
        aborts_if !exists<SupplyConfig>(aptos_addr);
        let post allow_upgrades_post = global<SupplyConfig>(@aptos_framework);
        ensures allow_upgrades_post.allow_upgrades == allowed;
    }

    spec balance {
        aborts_if !exists<CoinStore<CoinType>>(owner);
    }

    spec is_coin_initialized {
        pragma verify = false;
    }

    spec schema ExistCoinInfo<CoinType> {
        let addr = spec_coin_address<CoinType>();
        aborts_if !exists<CoinInfo<CoinType>>(addr);
    }

    spec name {
        pragma aborts_if_is_partial;
        include ExistCoinInfo<CoinType>;
    }

    spec symbol {
        pragma aborts_if_is_partial;
        include ExistCoinInfo<CoinType>;
    }

    spec decimals {
        pragma aborts_if_is_partial;
        include ExistCoinInfo<CoinType>;
    }

    spec supply {
        pragma aborts_if_is_partial;
        include ExistCoinInfo<CoinType>;
    }

    spec burn {
        pragma aborts_if_is_partial;
        include ExistCoinInfo<CoinType>;
        aborts_if coin.value == 0;
    }

    spec burn_from {
        pragma aborts_if_is_partial;
        aborts_if amount != 0 && !exists<CoinStore<CoinType>>(account_addr);
    }

    spec deposit {
        let coin_store = global<CoinStore<CoinType>>(account_addr);
        aborts_if !exists<CoinStore<CoinType>>(account_addr);
        aborts_if coin_store.frozen;
    }

    spec destroy_zero {
        aborts_if zero_coin.value > 0;
    }

    spec extract {
        aborts_if coin.value < amount;
        ensures coin.value == old(coin.value) - amount;
    }

    spec extract_all {
        ensures coin.value == 0;
    }

    spec freeze_coin_store {
        aborts_if !exists<CoinStore<CoinType>>(account_addr);
    }

    spec unfreeze_coin_store {
        aborts_if !exists<CoinStore<CoinType>>(account_addr);
    }

    spec upgrade_supply {
        pragma aborts_if_is_partial;
        let account_addr = signer::address_of(account);
        let coin_address = spec_coin_address<CoinType>();
        aborts_if coin_address != account_addr;
        aborts_if !exists<SupplyConfig>(@aptos_framework);
        aborts_if !exists<CoinInfo<CoinType>>(account_addr);
    }

    spec initialize {
        pragma verify = false;
    }

    spec initialize_with_parallelizable_supply {
        pragma aborts_if_is_partial;
        let addr = signer::address_of(account);
        aborts_if addr != @aptos_framework;
        include InitializeInternalSchema<CoinType>{
            name: name.bytes,
            symbol: symbol.bytes
        };
    }

    spec schema InitializeInternalSchema<CoinType> {
        account: signer;
        name: vector<u8>;
        symbol: vector<u8>;
        let account_addr = signer::address_of(account);
        let coin_address = spec_coin_address<CoinType>();
        aborts_if coin_address != account_addr;
        aborts_if exists<CoinInfo<CoinType>>(account_addr);
        aborts_if len(name) > MAX_COIN_NAME_LENGTH;
        aborts_if len(symbol) > MAX_COIN_SYMBOL_LENGTH;
    }

    spec initialize_internal {
        pragma aborts_if_is_partial;
        include InitializeInternalSchema<CoinType>{
            name: name.bytes,
            symbol: symbol.bytes
        };
    }

    spec merge {
        ensures dst_coin.value == old(dst_coin.value) + source_coin.value;
    }

    spec register {
        pragma aborts_if_is_partial;
        let account_addr = signer::address_of(account);
        aborts_if exists<CoinStore<CoinType>>(account_addr);
        aborts_if !exists<account::Account>(account_addr);
    }
    
    spec transfer {
        pragma verify = false;
    }

    spec withdraw {
        let account_addr = signer::address_of(account);
        let coin_store = global<CoinStore<CoinType>>(account_addr);
        let balance = coin_store.coin.value;
        aborts_if !exists<CoinStore<CoinType>>(account_addr);
        aborts_if coin_store.frozen;
        aborts_if balance < amount;

        let post coin_post = global<CoinStore<CoinType>>(account_addr).coin.value;
        ensures coin_post == balance - amount;
        ensures result == Coin<CoinType>{value: amount};
    }    
}