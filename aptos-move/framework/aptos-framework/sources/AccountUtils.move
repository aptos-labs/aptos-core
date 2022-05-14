module AptosFramework::AccountUtils {
    use AptosFramework::Account;
    use AptosFramework::Coin;
    use AptosFramework::TestCoin::TestCoin;

    public(script) fun create_and_fund_account(funder: &signer, account: address, amount: u64) {
        Account::create_account(account);
        Coin::transfer<TestCoin>(funder, account, amount);
    }
}
