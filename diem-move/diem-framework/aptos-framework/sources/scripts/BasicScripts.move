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

    public(script) fun mint(core_resource_account: signer, addr: address, amount: u64) {
        TestCoin::mint(&core_resource_account, addr, amount);
    }

    public(script) fun transfer(from: signer, to: address, amount: u64){
        TestCoin::transfer(&from, to, amount)
    }
}