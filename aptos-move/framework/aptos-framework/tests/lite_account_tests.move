#[test_only]
module aptos_framework::lite_account_tests {
    public fun test_auth(account: signer, _data: vector<u8>): signer { account }
}
