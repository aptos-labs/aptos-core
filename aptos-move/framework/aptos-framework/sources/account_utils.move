module aptos_framework::account_utils {
    use aptos_framework::account;
    use aptos_framework::coin;
    use aptos_framework::aptos_coin::AptosCoin;

    public entry fun create_and_fund_account(funder: &signer, account: address, amount: u64) {
        account::create_account(account);
        coin::transfer<AptosCoin>(funder, account, amount);
    }
}
