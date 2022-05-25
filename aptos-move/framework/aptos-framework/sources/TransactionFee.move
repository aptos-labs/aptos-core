module AptosFramework::TransactionFee {
    use AptosFramework::Coin::{Self, BurnCapability};
    use AptosFramework::TestCoin::TestCoin;

    friend AptosFramework::Account;

    struct TestCoinCapabilities has key {
        burn_cap: BurnCapability<TestCoin>,
    }

    /// Burn transaction fees in epilogue.
    public(friend) fun burn_fee(account: address, fee: u64) acquires TestCoinCapabilities {
        Coin::burn_from<TestCoin>(
            account,
            fee,
            &borrow_global<TestCoinCapabilities>(@AptosFramework).burn_cap,
        );
    }

    public fun store_test_coin_burn_cap(core_framework: &signer, burn_cap: BurnCapability<TestCoin>) {
        move_to(core_framework, TestCoinCapabilities { burn_cap })
    }
}
