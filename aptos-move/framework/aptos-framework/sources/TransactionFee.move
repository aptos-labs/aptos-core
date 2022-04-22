module AptosFramework::TransactionFee {
    use AptosFramework::TestCoin::{Self, Coin};

    friend AptosFramework::Account;

    /// Burn transaction fees in epilogue.
    public(friend) fun burn_fee(fee: Coin) {
        TestCoin::burn_gas(fee);
    }
}
