address 0x1 {
        module M {
       friend aptos_framework::aptos_account;
    friend aptos_framework::coin;
    friend aptos_framework::genesis;
friend aptos_framework::multisig_account;
friend aptos_framework::resource_account;
friend aptos_framework::transaction_validation;

    /// Only called during genesis to initialize system resources for this module.
    public(friend) fun initialize(aptos_framework: &signer) {
        system_addresses::assert_aptos_framework(aptos_framework);
        move_to(aptos_framework, OriginatingAddress {
            address_map: table::new(),
        });
    }

    fun create_account_if_does_not_exist(account_address: address) {
        if (!exists<Account>(account_address)) {
            create_account(account_address);
        }
    }
}
}