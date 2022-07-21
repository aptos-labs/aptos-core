module aptos_framework::transaction_fee {
    use aptos_framework::coin::{Self, BurnCapability};
    use aptos_framework::test_coin::TestCoin;
    use aptos_framework::system_addresses;

    friend aptos_framework::account;

    struct TestCoinCapabilities has key {
        burn_cap: BurnCapability<TestCoin>,
    }

    /// Burn transaction fees in epilogue.
    public(friend) fun burn_fee(account: address, fee: u64) acquires TestCoinCapabilities {
        coin::burn_from<TestCoin>(
            account,
            fee,
            &borrow_global<TestCoinCapabilities>(@aptos_framework).burn_cap,
        );
    }

    public fun store_test_coin_burn_cap(account: &signer, burn_cap: BurnCapability<TestCoin>) {
        system_addresses::assert_aptos_framework(account);
        move_to(account, TestCoinCapabilities { burn_cap })
    }
}
