module TestFunFormat {
    fun init_utility_coin_store<CoinType>(fee_account: &signer) {
        // Assert coin type corresponds to initialized coin.
        assert !(
            coin::is_coin_initialized<CoinType>(),
            E_NOT_COIN
        );
        // If a utility coin store does not already exist at account,
        if (!exists<UtilityCoinStore<CoinType>>(address_of(fee_account)))
            // Move to the fee account an initialized one.
            move_to<UtilityCoinStore<CoinType>>(
                fee_account,
                UtilityCoinStore {coins: coin::zero<CoinType>()}
            );
    }
}