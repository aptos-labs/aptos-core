/// This module allows for more convenient managing of coins across coin::CoinStore and
/// account::Account
module aptos_framework::coins {
    use std::signer;
    use aptos_framework::account;
    use aptos_framework::coin;

    /// Entry function to register to receive a specific `CoinType`. An account that wants to hold a coin type
    /// has to explicitly registers to do so. The register creates a special `CoinStore`
    /// to hold the specified `CoinType`.
    public entry fun register<CoinType>(account: &signer) {
        register_internal<CoinType>(account);
    }

    public fun register_internal<CoinType>(account: &signer) {
        coin::register<CoinType>(account);
        account::register_coin<CoinType>(signer::address_of(account));
    }
}
