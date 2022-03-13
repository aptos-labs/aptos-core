module AptosFramework::BasicScripts {
    use AptosFramework::AptosAccount;
    use AptosFramework::TestCoin;

    public(script) fun create_account(
        new_account_address: address,
        auth_key_prefix: vector<u8>,
    ) {
        let signer = AptosAccount::create_account(new_account_address, auth_key_prefix);
        TestCoin::register(&signer);
    }

    public(script) fun transfer(from: signer, to: address, amount: u64){
        TestCoin::transfer(&from, to, amount)
    }

    public(script) fun rotate_authentication_key(
        account: signer,
        new_authentication_key: vector<u8>,
    ) {
        AptosAccount::rotate_authentication_key(&account, new_authentication_key)
    }
}
