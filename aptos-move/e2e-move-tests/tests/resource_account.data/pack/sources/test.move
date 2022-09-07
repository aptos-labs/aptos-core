module 0x0b6beee9bc1ad3177403a04efeefb1901c12b7b575ac5124c0205efc0dd2e32a::test {
    use aptos_framework::account;
    use aptos_framework::resource_account;

    struct ModuleData has key {
        resource_signer_cap: account::SignerCapability,
    }

    fun init_module(account: &signer) {
        let resource_signer_cap = resource_account::retrieve_resource_account_cap(account, @0xcafe);
        move_to(account, ModuleData { resource_signer_cap });
    }
}